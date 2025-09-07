use bevy::{
    asset::{embedded_asset, load_internal_asset},
    core_pipeline::core_2d::graph::{Core2d, Node2d},
    ecs::entity::{EntityHashMap, EntityHashSet},
    prelude::*,
    render::{
        extract_component::UniformComponentPlugin,
        primitives::Aabb,
        render_graph::{RenderGraphApp, ViewNodeRunner},
        render_resource::{
            CachedRenderPipelineId, GpuArrayBufferable, PipelineCache, ShaderType,
            SpecializedRenderPipelines, StorageBuffer,
        },
        renderer::{RenderDevice, RenderQueue},
        sync_world::RenderEntity,
        texture::{CachedTexture, TextureCache},
        view::{
            ExtractedView, NoFrustumCulling, RenderVisibleEntities, ViewTarget, VisibilitySystems,
        },
        Extract, Render, RenderApp, RenderSet,
    },
};
use bevy_voronoi::prelude::{Voronoi2dPlugin, VoronoiCamera, VoronoiMaterial};

use crate::{
    node::{LightingLabel, LightingNode},
    pipeline::{
        Lighting2dCompositePipeline, Lighting2dPipelineKey, Lighting2dPrepassPipelines,
        TYPES_SHADER, VIEW_TRANSFORMATIONS_SHADER,
    },
    prelude::{AmbientLight2d, Lighting2dSettings, PointLight2d},
    types::{LightOccluder2d, PenetrationSettings, RaymarchSettings},
    util::create_aux_texture,
};

/// A plugin for adding 2D lighting in the Bevy engine.
///
/// This plugin sets up and configures the necessary components and systems for 2D lighting,
/// including [`AmbientLight2d`], [`Lighting2dSettings`], [`PointLight2d`], and [`LightOccluder2d`].
pub struct Lighting2dPlugin;

impl Plugin for Lighting2dPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, TYPES_SHADER, "shaders/types.wgsl", Shader::from_wgsl);
        load_internal_asset!(
            app,
            VIEW_TRANSFORMATIONS_SHADER,
            "shaders/view_transformations.wgsl",
            Shader::from_wgsl
        );
        embedded_asset!(app, "shaders/lighting.wgsl");
        embedded_asset!(app, "shaders/penetration.wgsl");
        embedded_asset!(app, "shaders/blur.wgsl");
        embedded_asset!(app, "shaders/composite.wgsl");

        app.add_plugins((
            UniformComponentPlugin::<ExtractedLighting2dSettings>::default(),
            Voronoi2dPlugin,
        ))
        .register_type::<AmbientLight2d>()
        .register_type::<PointLight2d>()
        .register_type::<LightOccluder2d>()
        .register_type::<Lighting2dSettings>()
        .add_systems(
            Update,
            (
                update_voronoi_camera,
                update_voronoi_material,
                remove_voronoi_material,
                remove_voronoi_camera,
            ),
        )
        .add_systems(
            PostUpdate,
            check_lighting_2d_artifacts_bounds.in_set(VisibilitySystems::CalculateBounds),
        );

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<SpecializedRenderPipelines<Lighting2dCompositePipeline>>()
            .add_systems(
                ExtractSchedule,
                (extract_lighting_settings, extract_point_lights),
            )
            .add_systems(
                Render,
                (
                    (prepare_lighting2d_textures, prepare_composite_pipelines)
                        .in_set(RenderSet::Prepare),
                    prepare_lighting2d_view_array_buffers::<ExtractedPointLight2d, PointLight2d>
                        .in_set(RenderSet::PrepareResources),
                ),
            )
            .add_render_graph_node::<ViewNodeRunner<LightingNode>>(Core2d, LightingLabel)
            .add_render_graph_edges(Core2d, (Node2d::EndMainPass, LightingLabel));
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .insert_resource(Lighing2dViewArrayBuffer::<ExtractedPointLight2d>::default())
            .init_resource::<Lighting2dPrepassPipelines>()
            .init_resource::<Lighting2dCompositePipeline>();
    }
}

fn update_voronoi_camera(
    mut query: Query<
        (&Lighting2dSettings, &mut VoronoiCamera),
        Or<(Added<Lighting2dSettings>, Changed<Lighting2dSettings>)>,
    >,
) {
    for (settings, mut voronoi_camera) in &mut query {
        voronoi_camera.down_sample = settings.down_sample
    }
}

fn remove_voronoi_camera(
    mut commands: Commands,
    mut removed: RemovedComponents<Lighting2dSettings>,
) {
    for entity in removed.read() {
        if let Ok(mut commands) = commands.get_entity(entity) {
            commands.remove::<VoronoiCamera>();
        }
    }
}

fn update_voronoi_material(
    mut query: Query<
        (&LightOccluder2d, &mut VoronoiMaterial),
        Or<(Added<LightOccluder2d>, Changed<LightOccluder2d>)>,
    >,
) {
    for (occluder, mut material) in &mut query {
        material.alpha_mask = occluder.occluder_mask.clone()
    }
}

fn remove_voronoi_material(
    mut commands: Commands,
    mut removed: RemovedComponents<LightOccluder2d>,
) {
    for entity in removed.read() {
        if let Ok(mut commands) = commands.get_entity(entity) {
            commands.remove::<VoronoiMaterial>();
        }
    }
}

pub fn check_lighting_2d_artifacts_bounds(
    mut commands: Commands,
    point_lights: Query<
        (Entity, &PointLight2d),
        (
            Or<(Without<Aabb>, Changed<PointLight2d>)>,
            Without<NoFrustumCulling>,
        ),
    >,
) {
    for (entity, point_light) in &point_lights {
        commands.entity(entity).try_insert(Aabb {
            center: Vec3::ZERO.into(),
            half_extents: Vec2::splat(point_light.radius).extend(0.).into(),
        });
    }
}

#[derive(Component, Clone, ShaderType)]
pub struct ExtractedLighting2dSettings {
    #[size(16)]
    pub raymarch: RaymarchSettings,
    pub penetration: PenetrationSettings,
    pub ambient_light: LinearRgba,
    pub down_sample: f32,
    pub tint_occluders: u32,
    pub edge_intensity: f32,
    pub blur: i32,
}

fn extract_lighting_settings(
    mut commands: Commands,
    ambient_light_query: Extract<
        Query<(RenderEntity, &Lighting2dSettings, &AmbientLight2d), With<Camera2d>>,
    >,
) {
    for (e, settings, ambient_light) in &ambient_light_query {
        commands.entity(e).insert(ExtractedLighting2dSettings {
            down_sample: settings.down_sample as f32,
            ambient_light: ambient_light.color.to_linear() * ambient_light.brightness,
            raymarch: settings.raymarch.clone(),
            penetration: settings.penetration.clone(),
            tint_occluders: if settings.tint_occluders { 1 } else { 0 },
            edge_intensity: settings.edge_intensity,
            blur: settings.blur as i32,
        });
    }
}

#[derive(Component, Default, Clone, ShaderType)]
pub struct ExtractedPointLight2d {
    pub center: Vec2,
    pub color: LinearRgba,
    pub falloff: f32,
    pub intensity: f32,
    pub radius: f32,
    pub shadows_enabled: u32,
}

fn extract_point_lights(
    mut commands: Commands,
    point_lights_query: Extract<
        Query<(
            RenderEntity,
            &PointLight2d,
            &GlobalTransform,
            &ViewVisibility,
        )>,
    >,
) {
    for (render_entity, point_light, transform, visibility) in point_lights_query.iter() {
        if !visibility.get() {
            continue;
        }

        commands
            .entity(render_entity)
            .insert(ExtractedPointLight2d {
                color: point_light.color.to_linear(),
                center: transform.translation().xy(),
                radius: point_light.radius,
                intensity: point_light.intensity,
                falloff: point_light.falloff,
                shadows_enabled: if point_light.shadows_enabled { 1 } else { 0 },
            });
    }
}

#[derive(Clone, Component)]
pub struct Lighting2dTexture {
    flip: bool,
    texture_a: CachedTexture,
    texture_b: CachedTexture,
}

impl Lighting2dTexture {
    pub fn input(&self) -> &CachedTexture {
        if self.flip {
            &self.texture_b
        } else {
            &self.texture_a
        }
    }

    pub fn output(&self) -> &CachedTexture {
        if self.flip {
            &self.texture_a
        } else {
            &self.texture_b
        }
    }

    pub fn flip(&mut self) {
        self.flip = !self.flip;
    }
}

fn prepare_lighting2d_textures(
    mut commands: Commands,
    view_query: Query<(Entity, &ViewTarget, &ExtractedLighting2dSettings)>,
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
) {
    for (entity, view_target, settings) in &view_query {
        commands.entity(entity).insert(Lighting2dTexture {
            flip: false,
            texture_a: create_aux_texture(
                view_target,
                &mut texture_cache,
                &render_device,
                "lighting2d_texture_a",
                settings.down_sample as u32,
            ),
            texture_b: create_aux_texture(
                view_target,
                &mut texture_cache,
                &render_device,
                "lighting2d_texture_b",
                settings.down_sample as u32,
            ),
        });
    }
}

#[derive(Component)]
pub struct Lighting2dCompositePipelineId(pub CachedRenderPipelineId);

fn prepare_composite_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut composite_pipelines: ResMut<SpecializedRenderPipelines<Lighting2dCompositePipeline>>,
    composite_pipeline: Res<Lighting2dCompositePipeline>,
    views_query: Query<(Entity, &ExtractedView), With<ExtractedLighting2dSettings>>,
) {
    for (entity, view) in &views_query {
        commands
            .entity(entity)
            .insert(Lighting2dCompositePipelineId(
                composite_pipelines.specialize(
                    &pipeline_cache,
                    &composite_pipeline,
                    Lighting2dPipelineKey { hdr: view.hdr },
                ),
            ));
    }
}

#[derive(ShaderType)]
pub struct Lighting2dArray<T: GpuArrayBufferable> {
    pub count: u32,
    #[size(runtime)]
    pub data: Vec<T>,
}

#[derive(Deref, DerefMut)]
pub struct Lighting2dArrayBuffer<T: GpuArrayBufferable>(StorageBuffer<Lighting2dArray<T>>);

impl<T: GpuArrayBufferable> Lighting2dArrayBuffer<T> {
    pub fn new(data: Vec<T>, count: u32) -> Self {
        Self(StorageBuffer::from(Lighting2dArray { data, count }))
    }
}

#[derive(Resource, Deref, DerefMut, Default)]
pub struct Lighing2dViewArrayBuffer<T: GpuArrayBufferable>(
    pub EntityHashMap<Lighting2dArrayBuffer<T>>,
);

fn prepare_lighting2d_view_array_buffers<T: Component + GpuArrayBufferable, U: Component>(
    mut view_entities: Local<EntityHashSet>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut view_array_buffer: ResMut<Lighing2dViewArrayBuffer<T>>,
    view_query: Query<(Entity, &RenderVisibleEntities), With<ExtractedLighting2dSettings>>,
    components: Query<(Entity, &T)>,
) {
    for (view_entity, visible_entities) in &view_query {
        view_entities.clear();

        for (e, _) in visible_entities.iter::<U>() {
            view_entities.insert(*e);
        }

        view_array_buffer.insert(
            view_entity,
            Lighting2dArrayBuffer::<T>::new(
                components
                    .iter()
                    .filter(|(entity, _)| view_entities.contains(entity))
                    .map(|(_, component)| component.clone())
                    .collect(),
                view_entities.len() as u32,
            ),
        );

        view_array_buffer
            .get_mut(&view_entity)
            .unwrap()
            .write_buffer(&render_device, &render_queue);
    }
}
