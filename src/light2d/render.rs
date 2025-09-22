use std::{hash::Hash, marker::PhantomData, ops::Range};

use bevy::{
    asset::{embedded_asset, load_embedded_asset},
    ecs::{
        entity::EntityHashMap,
        query::ROQueryItem,
        system::{
            lifetimeless::{Read, SRes},
            SystemParamItem,
        },
    },
    math::{Affine3A, FloatOrd},
    mesh::{VertexBufferLayout, VertexFormat},
    platform::collections::HashMap,
    prelude::*,
    render::{
        extract_component::{ComponentUniforms, DynamicUniformIndex},
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
            RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
        },
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, BlendComponent,
            BlendFactor, BlendOperation, BlendState, BufferUsages, ColorTargetState, ColorWrites,
            FragmentState, IndexFormat, PipelineCache, RawBufferVec, RenderPipelineDescriptor,
            SamplerBindingType, SamplerDescriptor, ShaderStages, SpecializedRenderPipeline,
            SpecializedRenderPipelines, TextureFormat, TextureSampleType, VertexState,
            VertexStepMode,
        },
        renderer::{RenderDevice, RenderQueue},
        sync_world::{MainEntity, RenderEntity},
        view::{
            ExtractedView, RenderVisibleEntities, RetainedViewEntity, ViewUniform,
            ViewUniformOffset, ViewUniforms,
        },
        Extract, Render, RenderApp, RenderStartup, RenderSystems,
    },
    shader::{load_shader_library, ShaderRef},
};
use bevy_voronoi::prelude::VoronoiTextures;
use bytemuck::{Pod, Zeroable};
use fixedbitset::FixedBitSet;

use crate::{post_process::render::ExtractedLighting2dSettings, render::Light2dPhase};

/// To be used in conjunction with [`CustomLight2dPlugin`]. It provides a high level way
/// to render 2d light entities with custom shader logic.
pub trait Light2dMaterial: Component + Default + Clone {
    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout;
    fn fragment_shader() -> ShaderRef;
    fn light_size(&self) -> Vec2;
    fn bind_group(&self, render_device: &RenderDevice, render_queue: &RenderQueue) -> BindGroup;
}

/// Adds the necessary ECS resources and render logic to enable rendering entities using
/// the given [`Light2dMaterial`] component types
#[derive(Default)]
pub struct CustomLight2dPlugin<L: Light2dMaterial>(PhantomData<L>);

impl<L: Light2dMaterial> Plugin for CustomLight2dPlugin<L> {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "light2d_common.wgsl");
        embedded_asset!(app, "light2d_vertex.wgsl");

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<RenderLights2dInstances<L>>()
            .init_resource::<SpecializedRenderPipelines<Light2dPipeline<L>>>()
            .init_resource::<Light2dMeta<L>>()
            .init_resource::<Light2dBatches<L>>()
            .init_resource::<Light2dMaterialBindGroups<L>>()
            .add_systems(ExtractSchedule, extract_light2d_instances::<L>)
            .add_systems(RenderStartup, init_light2d_pipeline::<L>)
            .add_systems(
                Render,
                (
                    queue_light2d_instances::<L>.in_set(RenderSystems::Queue),
                    (
                        prepare_light2d_view_bind_groups::<L>,
                        prepare_light2d_buffers::<L>,
                    )
                        .in_set(RenderSystems::PrepareBindGroups),
                ),
            )
            .add_render_command::<Light2dPhase, DrawLight2dMesh<L>>();
    }
}

#[derive(Resource)]
pub struct Light2dPipeline<L: Light2dMaterial> {
    vertex_shader: Handle<Shader>,
    fragment_shader: Option<Handle<Shader>>,
    view_layout: BindGroupLayout,
    light_layout: BindGroupLayout,
    marker: PhantomData<L>,
}

pub fn init_light2d_pipeline<L: Light2dMaterial>(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
) {
    commands.insert_resource(Light2dPipeline::<L> {
        vertex_shader: load_embedded_asset!(asset_server.as_ref(), "light2d_vertex.wgsl"),
        fragment_shader: match L::fragment_shader() {
            ShaderRef::Default => None,
            ShaderRef::Handle(handle) => Some(handle),
            ShaderRef::Path(path) => Some(asset_server.load(path)),
        },
        view_layout: render_device.create_bind_group_layout(
            "light2d_view_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::VERTEX_FRAGMENT,
                (
                    uniform_buffer::<ViewUniform>(true),
                    uniform_buffer::<ExtractedLighting2dSettings>(true)
                        .visibility(ShaderStages::FRAGMENT),
                    texture_2d(TextureSampleType::Float { filterable: true })
                        .visibility(ShaderStages::FRAGMENT),
                    sampler(SamplerBindingType::Filtering).visibility(ShaderStages::FRAGMENT),
                ),
            ),
        ),
        light_layout: L::bind_group_layout(&render_device),
        marker: PhantomData,
    });
}

#[derive(Eq, PartialEq, Hash, Clone, Copy)]
pub struct Light2dPipelineKey {
    pub hdr: bool,
}

impl<L: Light2dMaterial> SpecializedRenderPipeline for Light2dPipeline<L> {
    type Key = Light2dPipelineKey;

    fn specialize(&self, _key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("light2d_pipeline".into()),
            layout: vec![self.view_layout.clone(), self.light_layout.clone()],
            vertex: VertexState {
                shader: self.vertex_shader.clone(),
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
                        // @location(3) i_original_translation_rotation: vec4<f32>,
                        VertexFormat::Float32x4,
                    ],
                )],
            },
            fragment: match self.fragment_shader.clone() {
                Some(shader_handle) => Some(FragmentState {
                    shader: shader_handle,
                    shader_defs: vec![],
                    entry_point: Some("fragment".into()),
                    targets: vec![Some(ColorTargetState {
                        format: TextureFormat::Rgba16Float,
                        blend: Some(BlendState {
                            color: BlendComponent {
                                src_factor: BlendFactor::SrcAlpha,
                                dst_factor: BlendFactor::One,
                                operation: BlendOperation::Add,
                            },
                            alpha: BlendComponent {
                                src_factor: BlendFactor::One,
                                dst_factor: BlendFactor::One,
                                operation: BlendOperation::Add,
                            },
                        }),
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                None => None,
            },
            ..default()
        }
    }
}

pub struct ExtractedLight2d<L: Light2dMaterial> {
    pub transform: GlobalTransform,
    pub instance: L,
}

#[derive(Resource, Deref, DerefMut, Default)]
pub struct RenderLights2dInstances<L: Light2dMaterial>(
    HashMap<(Entity, MainEntity), ExtractedLight2d<L>>,
);

pub fn extract_light2d_instances<L: Light2dMaterial>(
    mut render_light_instances: ResMut<RenderLights2dInstances<L>>,
    light_query: Extract<Query<(Entity, RenderEntity, &ViewVisibility, &L, &GlobalTransform)>>,
) {
    render_light_instances.clear();

    for (entity, render_entity, view_visibility, light, transform) in light_query.iter() {
        if !view_visibility.get() {
            continue;
        }
        render_light_instances.insert(
            (render_entity, entity.into()),
            ExtractedLight2d::<L> {
                transform: *transform,
                instance: light.clone(),
            },
        );
    }
}

pub fn queue_light2d_instances<L: Light2dMaterial>(
    mut view_entities: Local<FixedBitSet>,
    draw_functions: Res<DrawFunctions<Light2dPhase>>,
    light2d_pipeline: Res<Light2dPipeline<L>>,
    mut pipelines: ResMut<SpecializedRenderPipelines<Light2dPipeline<L>>>,
    pipeline_cache: Res<PipelineCache>,
    render_light2d_instances: Res<RenderLights2dInstances<L>>,
    mut light2d_render_phases: ResMut<ViewSortedRenderPhases<Light2dPhase>>,
    mut views: Query<(&RenderVisibleEntities, &ExtractedView)>,
) {
    let draw_light_function = draw_functions.read().id::<DrawLight2dMesh<L>>();

    for (visible_entities, view) in &mut views {
        let Some(light2d_phase) = light2d_render_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };

        view_entities.clear();
        view_entities.extend(
            visible_entities
                .iter::<L>()
                .map(|(_, e)| e.index() as usize),
        );

        light2d_phase.items.reserve(render_light2d_instances.len());

        for ((render_entity, main_entity), render_light) in render_light2d_instances.iter() {
            let view_index = main_entity.index();

            if !view_entities.contains(view_index as usize) {
                continue;
            }

            let view_key = Light2dPipelineKey { hdr: view.hdr };

            let pipeline = pipelines.specialize(&pipeline_cache, &light2d_pipeline, view_key);

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

#[derive(Component, Deref, DerefMut)]
pub struct Light2dViewBindGroup(pub BindGroup);

pub fn prepare_light2d_view_bind_groups<L: Light2dMaterial>(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    light2d_pipeline: Res<Light2dPipeline<L>>,
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
    pub i_original_translation_rotation: [Vec2; 2],
}

impl Light2dInstance {
    fn from(transform: &GlobalTransform, light_size: &Vec2) -> Self {
        let affine = transform.affine()
            * Affine3A::from_scale_rotation_translation(
                light_size.extend(1.0),
                Quat::IDENTITY,
                (light_size * -Vec2::splat(0.5)).extend(0.0),
            );

        let transpose_model_3x3 = affine.matrix3.transpose();

        Self {
            i_model_transpose: [
                transpose_model_3x3.x_axis.extend(affine.translation.x),
                transpose_model_3x3.y_axis.extend(affine.translation.y),
                transpose_model_3x3.z_axis.extend(affine.translation.z),
            ],
            i_original_translation_rotation: [
                transform.translation().xy(),
                (-transform.rotation() * Vec3::Y).yx(),
            ],
        }
    }
}

#[derive(Resource)]
pub struct Light2dMeta<L: Light2dMaterial> {
    index_buffer: RawBufferVec<u32>,
    instance_buffer: RawBufferVec<Light2dInstance>,
    marker: PhantomData<L>,
}

impl<L: Light2dMaterial> Default for Light2dMeta<L> {
    fn default() -> Self {
        Self {
            index_buffer: RawBufferVec::<u32>::new(BufferUsages::INDEX),
            instance_buffer: RawBufferVec::<Light2dInstance>::new(BufferUsages::VERTEX),
            marker: PhantomData,
        }
    }
}

#[derive(Resource, Deref, DerefMut, Default)]
pub struct Light2dMaterialBindGroups<L: Light2dMaterial> {
    #[deref]
    pub bind_groups: EntityHashMap<BindGroup>,
    marker: PhantomData<L>,
}

#[derive(Resource, Deref, DerefMut, Default)]
pub struct Light2dBatches<L: Light2dMaterial> {
    #[deref]
    pub batches: HashMap<(RetainedViewEntity, Entity), Range<u32>>,
    marker: PhantomData<L>,
}

pub fn prepare_light2d_buffers<L: Light2dMaterial>(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    render_lights2d: Res<RenderLights2dInstances<L>>,
    mut light2d_bind_groups: ResMut<Light2dMaterialBindGroups<L>>,
    mut light2d_meta: ResMut<Light2dMeta<L>>,
    mut phases: ResMut<ViewSortedRenderPhases<Light2dPhase>>,
    mut batches: ResMut<Light2dBatches<L>>,
) {
    batches.clear();
    light2d_meta.instance_buffer.clear();

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

            light2d_bind_groups.insert(
                item.entity(),
                light.instance.bind_group(&render_device, &render_queue),
            );

            light2d_meta.instance_buffer.push(Light2dInstance::from(
                &light.transform,
                &light.instance.light_size(),
            ));

            current_batch.get_mut().end += 1;
            index += 1;

            phase.items[item_index].batch_range_mut().end += 1;
        }

        light2d_meta
            .instance_buffer
            .write_buffer(&render_device, &render_queue);
    }
}

pub type DrawLight2dMesh<L> = (
    SetItemPipeline,
    SetLight2dViewBindGroup<0>,
    SetLight2dMaterialBindGroup<L, 1>,
    DrawLight2dBatch<L>,
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

pub struct SetLight2dMaterialBindGroup<L: Light2dMaterial, const I: usize>(PhantomData<L>);
impl<P: PhaseItem, L: Light2dMaterial, const I: usize> RenderCommand<P>
    for SetLight2dMaterialBindGroup<L, I>
{
    type Param = SRes<Light2dMaterialBindGroups<L>>;
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

pub struct DrawLight2dBatch<L: Light2dMaterial>(pub PhantomData<L>);
impl<P: PhaseItem, L: Light2dMaterial> RenderCommand<P> for DrawLight2dBatch<L> {
    type Param = (SRes<Light2dMeta<L>>, SRes<Light2dBatches<L>>);
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
