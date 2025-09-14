use bevy::{
    core_pipeline::FullscreenShader,
    prelude::*,
    render::{
        render_resource::{
            binding_types::{sampler, storage_buffer_read_only, texture_2d, uniform_buffer},
            BindGroupLayout, BindGroupLayoutEntries, BindGroupLayoutEntry, CachedRenderPipelineId,
            ColorTargetState, ColorWrites, FragmentState, PipelineCache, RenderPipelineDescriptor,
            SamplerBindingType, ShaderStages, SpecializedRenderPipeline, TextureFormat,
            TextureSampleType,
        },
        renderer::RenderDevice,
        view::{ViewTarget, ViewUniform},
    },
};

use crate::plugin::{ExtractedLighting2dSettings, ExtractedPointLight2d, Lighting2dArray};

fn create_pipeline(
    render_device: &RenderDevice,
    pipeline_cache: &PipelineCache,
    fullscreen_shader: &FullscreenShader,
    label: &'static str,
    shader: Handle<Shader>,
    entries: &[BindGroupLayoutEntry],
) -> (BindGroupLayout, CachedRenderPipelineId) {
    let layout = render_device.create_bind_group_layout(
        &(String::from(label) + "_bind_group_layout") as &str,
        entries,
    );

    let pipeline = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some((String::from(label) + "_pipeline").into()),
        layout: vec![layout.clone()],
        vertex: fullscreen_shader.to_vertex_state(),
        fragment: Some(FragmentState {
            shader,
            shader_defs: vec![],
            entry_point: Some("fragment".into()),
            targets: vec![Some(ColorTargetState {
                format: TextureFormat::Rgba16Float,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
        }),
        push_constant_ranges: vec![],
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        zero_initialize_workgroup_memory: false,
    });

    (layout, pipeline)
}

pub fn init_lighting2d_pipelines(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    asset_server: Res<AssetServer>,
    fullscreen_shader: Res<FullscreenShader>,
) {
    let (lighting_layout, lighting_pipeline) = create_pipeline(
        &render_device,
        &pipeline_cache,
        &fullscreen_shader,
        "lighting",
        asset_server.load("embedded://bevy_lit/shaders/lighting.wgsl"),
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                uniform_buffer::<ViewUniform>(true),
                uniform_buffer::<ExtractedLighting2dSettings>(true),
                storage_buffer_read_only::<Lighting2dArray<ExtractedPointLight2d>>(false),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    );

    let (penetration_layout, penetration_pipeline) = create_pipeline(
        &render_device,
        &pipeline_cache,
        &fullscreen_shader,
        "penetration",
        asset_server.load("embedded://bevy_lit/shaders/penetration.wgsl"),
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                uniform_buffer::<ViewUniform>(true),
                uniform_buffer::<ExtractedLighting2dSettings>(true),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    );

    let (blur_layout, blur_pipeline) = create_pipeline(
        &render_device,
        &pipeline_cache,
        &fullscreen_shader,
        "blur",
        asset_server.load("embedded://bevy_lit/shaders/blur.wgsl"),
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                uniform_buffer::<ExtractedLighting2dSettings>(true),
                uniform_buffer::<IVec2>(false),
                texture_2d(TextureSampleType::Float { filterable: true }),
            ),
        ),
    );

    commands.insert_resource(Lighting2dPipelines {
        lighting_layout,
        lighting_pipeline,
        penetration_layout,
        penetration_pipeline,
        blur_layout,
        blur_pipeline,
    });
}

#[derive(Resource)]
pub struct Lighting2dPipelines {
    pub lighting_layout: BindGroupLayout,
    pub lighting_pipeline: CachedRenderPipelineId,
    pub penetration_layout: BindGroupLayout,
    pub penetration_pipeline: CachedRenderPipelineId,
    pub blur_layout: BindGroupLayout,
    pub blur_pipeline: CachedRenderPipelineId,
}

#[derive(Resource)]
pub struct Lighting2dCompositePipeline {
    pub layout: BindGroupLayout,
    pub shader: Handle<Shader>,
    pub fullscreen_shader: FullscreenShader,
}

pub fn init_lighting2d_composite_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
    fullscreen_shader: Res<FullscreenShader>,
) {
    commands.insert_resource(Lighting2dCompositePipeline {
        shader: asset_server.load("embedded://bevy_lit/shaders/composite.wgsl"),
        fullscreen_shader: fullscreen_shader.clone(),
        layout: render_device.create_bind_group_layout(
            "composite_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    uniform_buffer::<ExtractedLighting2dSettings>(true),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        ),
    });
}

#[derive(Eq, PartialEq, Hash, Clone, Copy)]
pub struct Lighting2dPipelineKey {
    pub hdr: bool,
}

impl SpecializedRenderPipeline for Lighting2dCompositePipeline {
    type Key = Lighting2dPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("composite_pipeline".into()),
            layout: vec![self.layout.clone()],
            vertex: self.fullscreen_shader.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs: vec![],
                entry_point: Some("fragment".into()),
                targets: vec![Some(ColorTargetState {
                    format: if key.hdr {
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
