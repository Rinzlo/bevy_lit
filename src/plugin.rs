use bevy::{
    asset::embedded_asset,
    core_pipeline::core_2d::{
        extract_core_2d_camera_phases,
        graph::{Core2d, Node2d},
    },
    prelude::*,
    render::{
        extract_component::UniformComponentPlugin,
        render_graph::{RenderGraphExt, ViewNodeRunner},
        render_phase::{AddRenderCommand, DrawFunctions, ViewSortedRenderPhases},
        render_resource::SpecializedRenderPipelines,
        Render, RenderApp, RenderStartup, RenderSystems,
    },
    shader::load_shader_library,
};
use bevy_voronoi::prelude::{Voronoi2dPlugin, VoronoiMaterial, VoronoiView};

use crate::{
    light2d::{
        node::{extract_light2d_phases, Light2dDrawNode, Light2dDrawPassLabel, Light2dPhase},
        render::{
            extract_light2d_instances, init_light2d_pipeline, prepare_light2d_buffers,
            prepare_light2d_view_bind_groups, prepare_lighting_textures, queue_light2d_instances,
            DrawLight2dMesh, Light2dBatches, Light2dMaterialBindGroups, Light2dMeta,
            Light2dPipeline, LightingTextures, RenderLights2dInstances,
        },
    },
    post_process::{
        lighting_settings_2d::Lighting2dSettings,
        node::{Light2dPostProcessDrawNode, Light2dPostProcessPassLabel},
        render::{
            extract_lighting_settings, init_lighting2d_composite_pipeline,
            init_post_process_pipelines, prepare_composite_pipelines, ExtractedLighting2dSettings,
            Lighting2dCompositePipeline,
        },
    },
};

/// A plugin for adding 2D lighting in the Bevy engine.
///
/// This plugin sets up and configures the necessary components and systems for 2D lighting,
/// including [`AmbientLight2d`], [`Lighting2dSettings`], [`PointLight2d`], and [`LightOccluder2d`].
pub struct Lighting2dPlugin;

impl Plugin for Lighting2dPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "shaders/types.wgsl");
        load_shader_library!(app, "shaders/view_transformations.wgsl");
        embedded_asset!(app, "shaders/lighting.wgsl");
        embedded_asset!(app, "shaders/penetration.wgsl");
        embedded_asset!(app, "shaders/blur.wgsl");
        embedded_asset!(app, "shaders/composite.wgsl");
        embedded_asset!(app, "light2d/light2d.wgsl");

        app.add_plugins((
            UniformComponentPlugin::<ExtractedLighting2dSettings>::default(),
            Voronoi2dPlugin,
        ))
        .add_systems(
            Update,
            (
                update_voronoi_view,
                update_voronoi_material,
                remove_voronoi_material,
                remove_voronoi_view,
            ),
        );

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<SpecializedRenderPipelines<Lighting2dCompositePipeline>>()
            .init_resource::<ViewSortedRenderPhases<Light2dPhase>>()
            .init_resource::<RenderLights2dInstances>()
            .init_resource::<DrawFunctions<Light2dPhase>>()
            .init_resource::<SpecializedRenderPipelines<Light2dPipeline>>()
            .init_resource::<Light2dMeta>()
            .init_resource::<Light2dBatches>()
            .init_resource::<Light2dMaterialBindGroups>()
            .init_resource::<LightingTextures>()
            .add_systems(
                ExtractSchedule,
                (
                    extract_lighting_settings,
                    extract_light2d_phases.after(extract_core_2d_camera_phases),
                    extract_light2d_instances,
                ),
            )
            .add_systems(
                RenderStartup,
                (
                    init_post_process_pipelines,
                    init_lighting2d_composite_pipeline,
                    init_light2d_pipeline,
                ),
            )
            .add_systems(
                Render,
                (
                    prepare_composite_pipelines.in_set(RenderSystems::Prepare),
                    queue_light2d_instances.in_set(RenderSystems::Queue),
                    (
                        prepare_lighting_textures,
                        prepare_light2d_view_bind_groups,
                        prepare_light2d_buffers,
                    )
                        .in_set(RenderSystems::PrepareBindGroups),
                ),
            )
            .add_render_command::<Light2dPhase, DrawLight2dMesh>()
            .add_render_graph_node::<ViewNodeRunner<Light2dDrawNode>>(Core2d, Light2dDrawPassLabel)
            .add_render_graph_node::<ViewNodeRunner<Light2dPostProcessDrawNode>>(
                Core2d,
                Light2dPostProcessPassLabel,
            )
            .add_render_graph_edges(
                Core2d,
                (
                    Node2d::EndMainPass,
                    Light2dDrawPassLabel,
                    Light2dPostProcessPassLabel,
                ),
            );
    }
}

/// A light occluder component. Should be used alongside a Mesh2d
#[derive(Component, Clone, Debug, Default, Reflect)]
#[require(VoronoiMaterial)]
pub struct LightOccluder2d {
    /// Any texture with a transparent background. The occluder will take it's shape.
    pub occluder_mask: Handle<Image>,
}

impl LightOccluder2d {
    pub fn new(occluder_mask: Handle<Image>) -> Self {
        Self { occluder_mask }
    }
}

fn update_voronoi_view(
    mut query: Query<
        (&Lighting2dSettings, &mut VoronoiView),
        Or<(Added<Lighting2dSettings>, Changed<Lighting2dSettings>)>,
    >,
) {
    for (settings, mut voronoi_view) in &mut query {
        voronoi_view.scale = settings.scale;
    }
}

fn remove_voronoi_view(mut commands: Commands, mut removed: RemovedComponents<Lighting2dSettings>) {
    for entity in removed.read() {
        if let Ok(mut commands) = commands.get_entity(entity) {
            commands.remove::<VoronoiView>();
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
