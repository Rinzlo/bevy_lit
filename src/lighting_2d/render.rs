use std::ops::Range;

use bevy::{
    asset::load_embedded_asset,
    ecs::{
        entity::EntityHashMap,
        query::ROQueryItem,
        system::{
            lifetimeless::{Read, SRes},
            SystemParamItem,
        },
    },
    image::BevyDefault,
    math::{Affine3A, FloatOrd},
    mesh::{VertexBufferLayout, VertexFormat},
    platform::collections::{HashMap, HashSet},
    prelude::*,
    render::{
        extract_component::{ComponentUniforms, DynamicUniformIndex},
        render_phase::{
            DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand, RenderCommandResult,
            SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
        },
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, BufferUsages,
            ColorTargetState, ColorWrites, Extent3d, FragmentState, IndexFormat, PipelineCache,
            RawBufferVec, RenderPipelineDescriptor, SamplerBindingType, SamplerDescriptor,
            ShaderStages, ShaderType, SpecializedRenderPipeline, SpecializedRenderPipelines,
            TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
            UniformBuffer, VertexState, VertexStepMode,
        },
        renderer::{RenderDevice, RenderQueue},
        sync_world::{MainEntity, RenderEntity},
        texture::{CachedTexture, TextureCache},
        view::{
            ExtractedView, RenderVisibleEntities, RetainedViewEntity, ViewTarget, ViewUniform,
            ViewUniformOffset, ViewUniforms,
        },
        Extract,
    },
};
use bevy_voronoi::prelude::VoronoiTextures;
use bytemuck::{Pod, Zeroable};
use fixedbitset::FixedBitSet;

use crate::{
    lighting_2d::{light_2d::Light2d, node::Light2dPhase},
    plugin::ExtractedLighting2dSettings,
};

#[derive(Resource)]
pub struct Light2dPipeline {
    shader: Handle<Shader>,
    view_layout: BindGroupLayout,
    point_material_layout: BindGroupLayout,
}

pub fn init_light2d_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
) {
    let view_layout = render_device.create_bind_group_layout(
        "light2d_view_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::VERTEX_FRAGMENT,
            (
                uniform_buffer::<ViewUniform>(true),
                uniform_buffer::<ExtractedLighting2dSettings>(true)
                    .visibility(ShaderStages::FRAGMENT),
                // lighting texture
                texture_2d(TextureSampleType::Float { filterable: true })
                    .visibility(ShaderStages::FRAGMENT),
                sampler(SamplerBindingType::Filtering).visibility(ShaderStages::FRAGMENT),
            ),
        ),
    );

    let point_layout = render_device.create_bind_group_layout(
        "point_light2d_layout",
        &BindGroupLayoutEntries::single(
            ShaderStages::FRAGMENT,
            uniform_buffer::<PointLight2dGpuType>(false),
        ),
    );

    commands.insert_resource(Light2dPipeline {
        shader: load_embedded_asset!(asset_server.as_ref(), "light_2d.wgsl"),
        view_layout,
        point_material_layout: point_layout,
    });
}

#[derive(Eq, PartialEq, Hash, Clone, Copy, Reflect)]
pub enum Light2dType {
    Point,
}

#[derive(Eq, PartialEq, Hash, Clone, Copy)]
pub struct Light2dPipelineKey {
    pub hdr: bool,
    pub light2d_type: Light2dType,
}

impl SpecializedRenderPipeline for Light2dPipeline {
    type Key = Light2dPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("light2d_pipeline".into()),
            layout: vec![
                self.view_layout.clone(),
                match key.light2d_type {
                    Light2dType::Point => self.point_material_layout.clone(),
                },
            ],
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: vec![],
                entry_point: Some("vertex".into()),
                buffers: vec![VertexBufferLayout::from_vertex_formats(
                    VertexStepMode::Instance,
                    vec![
                        // @location(0) i_model_transpose_col0: vec4<f32>,
                        VertexFormat::Float32x4,
                        // @location(1) i_model_transpose_col1: vec4<f32>,
                        VertexFormat::Float32x4,
                        // @location(2) i_model_transpose_col2: vec4<f32>,
                        VertexFormat::Float32x4,
                        // @location(3) i_model_color: vec4<f32>,
                        VertexFormat::Float32x4,
                    ],
                )],
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs: vec![],
                entry_point: Some("fragment".into()),
                targets: vec![Some(ColorTargetState {
                    format: if true {
                        ViewTarget::TEXTURE_FORMAT_HDR
                    } else {
                        TextureFormat::bevy_default()
                    },
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            push_constant_ranges: vec![],
            zero_initialize_workgroup_memory: false,
        }
    }
}

pub struct ExtractedLight2d {
    pub transform: GlobalTransform,
    pub color: LinearRgba,
    pub shadows_enabled: u32,
    pub kind: ExtractedLight2dKind,
}

pub enum ExtractedLight2dKind {
    Point { falloff: f32, radius: f32 },
}

#[derive(Resource, Deref, DerefMut, Default)]
pub struct RenderLights2dInstances(HashMap<(Entity, MainEntity), ExtractedLight2d>);

pub fn extract_light2d_instances(
    mut render_light_instances: ResMut<RenderLights2dInstances>,
    light_query: Extract<
        Query<(
            Entity,
            RenderEntity,
            &ViewVisibility,
            &Light2d,
            &GlobalTransform,
        )>,
    >,
) {
    render_light_instances.clear();

    for (entity, render_entity, view_visibility, light, transform) in light_query.iter() {
        if !view_visibility.get() {
            continue;
        }

        let (kind, color, shadows_enabled) = match light {
            Light2d::Point {
                color,
                intensity,
                shadows_enabled,
                radius,
                falloff,
            } => (
                ExtractedLight2dKind::Point {
                    radius: *radius,
                    falloff: *falloff,
                },
                color.to_linear() * *intensity,
                *shadows_enabled,
            ),
        };

        render_light_instances.insert(
            (render_entity, entity.into()),
            ExtractedLight2d {
                kind,
                color,
                shadows_enabled: if shadows_enabled { 1 } else { 0 },
                transform: *transform,
            },
        );
    }
}

pub fn queue_light2d_instances(
    mut view_entities: Local<FixedBitSet>,
    draw_functions: Res<DrawFunctions<Light2dPhase>>,
    light2d_pipeline: Res<Light2dPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<Light2dPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    render_light2d_instances: Res<RenderLights2dInstances>,
    mut light2d_render_phases: ResMut<ViewSortedRenderPhases<Light2dPhase>>,
    mut views: Query<(&RenderVisibleEntities, &ExtractedView)>,
) {
    let draw_light_function = draw_functions.read().id::<DrawLight2dMesh>();

    for (visible_entities, view) in &mut views {
        let Some(light2d_phase) = light2d_render_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };

        let view_key = Light2dPipelineKey {
            hdr: view.hdr,
            light2d_type: Light2dType::Point,
        };

        let pipeline = pipelines.specialize(&pipeline_cache, &light2d_pipeline, view_key);

        view_entities.clear();
        view_entities.extend(
            visible_entities
                .iter::<Light2d>()
                .map(|(_, e)| e.index() as usize),
        );

        light2d_phase.items.reserve(render_light2d_instances.len());

        for ((render_entity, main_entity), render_light) in render_light2d_instances.iter() {
            let view_index = main_entity.index();

            if !view_entities.contains(view_index as usize) {
                continue;
            }

            light2d_phase.add(Light2dPhase {
                draw_function: draw_light_function,
                pipeline,
                entity: (*render_entity, *main_entity),
                sort_key: FloatOrd(render_light.transform.translation().z),
                // `batch_range` is calculated in `prepare_light2d_buffers`
                batch_range: 0..0,
                extra_index: PhaseItemExtraIndex::None,
                indexed: true,
            });
        }
    }
}

#[derive(Clone)]
pub struct LightingTexture {
    flip: bool,
    texture_a: CachedTexture,
    texture_b: CachedTexture,
}

#[derive(Resource, Deref, DerefMut, Default)]
pub struct LightingTextures(pub HashMap<RetainedViewEntity, LightingTexture>);

impl LightingTexture {
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

fn create_aux_texture(
    view_target: &ViewTarget,
    texture_cache: &mut TextureCache,
    render_device: &RenderDevice,
    label: &'static str,
    scale: f32,
) -> CachedTexture {
    let size = view_target.main_texture().size();
    let size = Extent3d {
        width: (size.width as f32 * scale) as u32,
        height: (size.height as f32 * scale) as u32,
        depth_or_array_layers: size.depth_or_array_layers,
    };

    texture_cache.get(
        render_device,
        TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba16Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
    )
}

pub fn prepare_lighting_textures(
    views: Query<(&ViewTarget, &ExtractedView, &ExtractedLighting2dSettings)>,
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    mut lighting_textures: ResMut<LightingTextures>,
    mut live_entities: Local<HashSet<RetainedViewEntity>>,
) {
    live_entities.clear();

    for (view_target, extracted_view, settings) in &views {
        live_entities.insert(extracted_view.retained_view_entity);

        lighting_textures.insert(
            extracted_view.retained_view_entity,
            LightingTexture {
                flip: false,
                texture_a: create_aux_texture(
                    view_target,
                    &mut texture_cache,
                    &render_device,
                    "lighting_texture_a",
                    settings.scale,
                ),
                texture_b: create_aux_texture(
                    view_target,
                    &mut texture_cache,
                    &render_device,
                    "lighting_texture_b",
                    settings.scale,
                ),
            },
        );
    }

    lighting_textures.retain(|entity, _| live_entities.contains(entity));
}

#[derive(Component, Deref, DerefMut)]
pub struct Light2dViewBindGroup(pub BindGroup);

pub fn prepare_light2d_view_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    light2d_pipeline: Res<Light2dPipeline>,
    view_uniforms: Res<ViewUniforms>,
    voronoi_textures: Res<VoronoiTextures>,
    lighting2d_settings: Res<ComponentUniforms<ExtractedLighting2dSettings>>,
    views: Query<(Entity, &ExtractedView)>,
) {
    let (Some(view_binding), Some(lighting_settings_binding)) = (
        view_uniforms.uniforms.binding(),
        lighting2d_settings.binding(),
    ) else {
        return;
    };

    let sampler = render_device.create_sampler(&SamplerDescriptor::default());

    for (entity, view) in &views {
        let Some(voronoi_texture) = voronoi_textures.get(&view.retained_view_entity) else {
            continue;
        };

        let view_bind_group = render_device.create_bind_group(
            "light2d_view_bind_group",
            &light2d_pipeline.view_layout,
            &BindGroupEntries::sequential((
                view_binding.clone(),
                lighting_settings_binding.clone(),
                &voronoi_texture.input().default_view,
                &sampler.clone(),
            )),
        );

        commands
            .entity(entity)
            .insert(Light2dViewBindGroup(view_bind_group));
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
struct Light2dInstance {
    // Affine 4x3 transposed to 3x4
    pub i_model_transpose: [Vec4; 3],
    pub i_color: [f32; 4],
}

impl Light2dInstance {
    fn from(transform: &Affine3A, color: &LinearRgba) -> Self {
        let transpose_model_3x3 = transform.matrix3.transpose();

        Self {
            i_model_transpose: [
                transpose_model_3x3.x_axis.extend(transform.translation.x),
                transpose_model_3x3.y_axis.extend(transform.translation.y),
                transpose_model_3x3.z_axis.extend(transform.translation.z),
            ],
            i_color: color.to_f32_array(),
        }
    }
}

#[derive(Resource)]
pub struct Light2dMeta {
    index_buffer: RawBufferVec<u32>,
    instance_buffer: RawBufferVec<Light2dInstance>,
}

impl Default for Light2dMeta {
    fn default() -> Self {
        Self {
            index_buffer: RawBufferVec::<u32>::new(BufferUsages::INDEX),
            instance_buffer: RawBufferVec::<Light2dInstance>::new(BufferUsages::VERTEX),
        }
    }
}

#[derive(ShaderType)]
pub struct PointLight2dGpuType {
    center: Vec2,
    radius: f32,
    falloff: f32,
    shadows_enabled: u32,
}

#[derive(Resource, Deref, DerefMut, Default)]
pub struct Light2dMaterialBindGroups(pub EntityHashMap<BindGroup>);

#[derive(Resource, Deref, DerefMut, Default)]
pub struct Light2dBatches(pub HashMap<(RetainedViewEntity, Entity), Range<u32>>);

pub fn prepare_light2d_buffers(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    light2d_pipeline: Res<Light2dPipeline>,
    render_lights2d: Res<RenderLights2dInstances>,
    mut light2d_bind_groups: ResMut<Light2dMaterialBindGroups>,
    mut light2d_meta: ResMut<Light2dMeta>,
    mut phases: ResMut<ViewSortedRenderPhases<Light2dPhase>>,
    mut batches: ResMut<Light2dBatches>,
) {
    batches.clear();
    // Clear the light2d instances
    light2d_meta.instance_buffer.clear();

    // Index buffer indices
    let mut index = 0;

    for (retained_view, phase) in phases.iter_mut() {
        for item_index in 0..phase.items.len() {
            let item = &phase.items[item_index];

            let Some(light) = render_lights2d.get(&(item.entity(), item.main_entity())) else {
                continue;
            };

            let mut current_batch = batches
                .entry((*retained_view, item.entity()))
                .insert(index..index);

            let (quad_size, light_bind_group) = match light.kind {
                ExtractedLight2dKind::Point { radius, falloff } => {
                    let mut buffer = UniformBuffer::from(PointLight2dGpuType {
                        radius,
                        falloff,
                        center: light.transform.translation().xy(),
                        shadows_enabled: light.shadows_enabled,
                    });

                    buffer.write_buffer(&render_device, &render_queue);

                    let light_bind_group = render_device.create_bind_group(
                        "point_light_2d_bind_group",
                        &light2d_pipeline.point_material_layout,
                        &BindGroupEntries::single(buffer.binding().unwrap()),
                    );

                    (Vec2::splat(radius * 2.0), light_bind_group)
                }
            };
            let transform = light.transform.affine()
                * Affine3A::from_scale_rotation_translation(
                    quad_size.extend(1.0),
                    Quat::IDENTITY,
                    (quad_size * -Vec2::splat(0.5)).extend(0.0),
                );

            light2d_bind_groups.insert(item.entity(), light_bind_group);

            // Store the vertex data and add the item to the render phase
            light2d_meta
                .instance_buffer
                .push(Light2dInstance::from(&transform, &light.color));

            current_batch.get_mut().end += 1;
            index += 1;

            phase.items[item_index].batch_range_mut().end += 1;
        }

        light2d_meta
            .instance_buffer
            .write_buffer(&render_device, &render_queue);

        if light2d_meta.index_buffer.len() != 6 {
            light2d_meta.index_buffer.clear();

            // NOTE: This code is creating 6 indices pointing to 4 vertices.
            light2d_meta.index_buffer.push(2);
            light2d_meta.index_buffer.push(0);
            light2d_meta.index_buffer.push(1);
            light2d_meta.index_buffer.push(1);
            light2d_meta.index_buffer.push(3);
            light2d_meta.index_buffer.push(2);

            light2d_meta
                .index_buffer
                .write_buffer(&render_device, &render_queue);
        }
    }
}

pub type DrawLight2dMesh = (
    SetItemPipeline,
    SetLight2dViewBindGroup<0>,
    SetLight2dMaterialBindGroup<1>,
    DrawLight2dBatch,
);

pub struct SetLight2dViewBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetLight2dViewBindGroup<I> {
    type Param = ();
    type ViewQuery = (
        Read<Light2dViewBindGroup>,
        Read<ViewUniformOffset>,
        Read<DynamicUniformIndex<ExtractedLighting2dSettings>>,
    );
    type ItemQuery = ();

    fn render<'w>(
        _item: &P,
        (light2d_view_bind_group, view_uniform, light2d_settings_uniform_index): ROQueryItem<
            'w,
            '_,
            Self::ViewQuery,
        >,
        _entity: Option<()>,
        _param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(
            I,
            &light2d_view_bind_group,
            &[view_uniform.offset, light2d_settings_uniform_index.index()],
        );
        RenderCommandResult::Success
    }
}

pub struct SetLight2dMaterialBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetLight2dMaterialBindGroup<I> {
    type Param = SRes<Light2dMaterialBindGroups>;
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _item_query: Option<()>,
        bind_groups: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let bind_groups = bind_groups.into_inner();
        let Some(bind_group) = bind_groups.get(&item.entity()) else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(I, &bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub struct DrawLight2dBatch;
impl<P: PhaseItem> RenderCommand<P> for DrawLight2dBatch {
    type Param = (SRes<Light2dMeta>, SRes<Light2dBatches>);
    type ViewQuery = Read<ExtractedView>;
    type ItemQuery = ();

    fn render<'w>(
        item: &P,
        view: ROQueryItem<'w, '_, Self::ViewQuery>,
        _entity: Option<()>,
        (light2d_meta, batches): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let light2d_meta = light2d_meta.into_inner();
        let Some(batch) = batches.get(&(view.retained_view_entity, item.entity())) else {
            return RenderCommandResult::Skip;
        };

        pass.set_index_buffer(
            light2d_meta.index_buffer.buffer().unwrap().slice(..),
            0,
            IndexFormat::Uint32,
        );
        pass.set_vertex_buffer(0, light2d_meta.instance_buffer.buffer().unwrap().slice(..));
        pass.draw_indexed(0..6, 0, batch.clone());

        RenderCommandResult::Success
    }
}
