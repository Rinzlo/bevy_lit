use bevy::{
    asset::{load_embedded_asset, Handle},
    core_pipeline::FullscreenShader,
    prelude::*,
    render::{
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            BindGroupLayoutDescriptor, BindGroupLayoutEntries, BindGroupLayoutEntry,
            CachedRenderPipelineId, ColorTargetState, ColorWrites, FragmentState, PipelineCache,
            RenderPipelineDescriptor, SamplerBindingType, ShaderStages, ShaderType,
            SpecializedRenderPipeline, SpecializedRenderPipelines, TextureFormat,
            TextureSampleType,
        },
        sync_world::RenderEntity,
        view::{ExtractedView, ViewTarget, ViewUniform},
        Extract,
    },
    shader::Shader,
};

use crate::settings::{AmbientLight2d, Lighting2dSettings, PenetrationSettings, RaymarchSettings};

#[derive(Resource)]
pub struct Lighting2dPostProcessPipelines {
    pub penetration_layout_desc: BindGroupLayoutDescriptor,
    pub penetration_pipeline: CachedRenderPipelineId,
    pub blur_layout_desc: BindGroupLayoutDescriptor,
    pub blur_pipeline: CachedRenderPipelineId,
}

fn create_post_process_pipeline(
    pipeline_cache: &PipelineCache,
    fullscreen_shader: &FullscreenShader,
    label: &'static str,
    shader: Handle<Shader>,
    entries: &[BindGroupLayoutEntry],
) -> (BindGroupLayoutDescriptor, CachedRenderPipelineId) {
    let layout_desc =
        BindGroupLayoutDescriptor::new(String::from(label) + "_bind_group_layout", entries);

    let pipeline = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some((String::from(label) + "_pipeline").into()),
        layout: vec![layout_desc.clone()],
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

    (layout_desc, pipeline)
}

pub fn init_post_process_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    asset_server: Res<AssetServer>,
    fullscreen_shader: Res<FullscreenShader>,
) {
    let (penetration_layout_desc, penetration_pipeline) = create_post_process_pipeline(
        &pipeline_cache,
        &fullscreen_shader,
        "penetration",
        load_embedded_asset!(asset_server.as_ref(), "penetration.wgsl"),
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                uniform_buffer::<ViewUniform>(true),
                uniform_buffer::<ExtractedLighting2dSettings>(true),
                texture_2d(TextureSampleType::Float { filterable: true }),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    );

    let (blur_layout_desc, blur_pipeline) = create_post_process_pipeline(
        &pipeline_cache,
        &fullscreen_shader,
        "blur",
        load_embedded_asset!(asset_server.as_ref(), "blur.wgsl"),
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                uniform_buffer::<ExtractedLighting2dSettings>(true),
                uniform_buffer::<IVec2>(false),
                texture_2d(TextureSampleType::Float { filterable: true }),
            ),
        ),
    );

    commands.insert_resource(Lighting2dPostProcessPipelines {
        penetration_layout_desc,
        penetration_pipeline,
        blur_layout_desc,
        blur_pipeline,
    });
}

#[derive(Resource)]
pub struct Lighting2dCompositePipeline {
    pub layout_desc: BindGroupLayoutDescriptor,
    pub shader: Handle<Shader>,
    pub fullscreen_shader: FullscreenShader,
}

pub fn init_lighting2d_composite_pipeline(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    fullscreen_shader: Res<FullscreenShader>,
) {
    let layout_label = "composite_bind_group_layout";
    let layout_entries = BindGroupLayoutEntries::sequential(
        ShaderStages::FRAGMENT,
        (
            uniform_buffer::<ExtractedLighting2dSettings>(true),
            texture_2d(TextureSampleType::Float { filterable: true }),
            texture_2d(TextureSampleType::Float { filterable: true }),
            sampler(SamplerBindingType::Filtering),
        ),
    );
    commands.insert_resource(Lighting2dCompositePipeline {
        shader: load_embedded_asset!(asset_server.as_ref(), "composite.wgsl"),
        fullscreen_shader: fullscreen_shader.clone(),
        layout_desc: BindGroupLayoutDescriptor::new(layout_label, &layout_entries),
    });
}

#[derive(Eq, PartialEq, Hash, Clone, Copy)]
pub struct Lighting2dPipelineKey {
    pub hdr: bool,
    pub msaa_samples: u32,
}

impl SpecializedRenderPipeline for Lighting2dCompositePipeline {
    type Key = Lighting2dPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("composite_pipeline".into()),
            layout: vec![self.layout_desc.clone()],
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

#[derive(Component, Clone, ShaderType)]
pub struct ExtractedLighting2dSettings {
    #[shader(size(16))]
    pub raymarch: RaymarchSettings,
    pub penetration: PenetrationSettings,
    pub ambient_light: LinearRgba,
    pub scale: f32,
    pub tint_occluders: u32,
    pub edge_intensity: f32,
    pub blur: i32,
}

pub fn extract_lighting2d_settings(
    mut commands: Commands,
    ambient_light_query: Extract<
        Query<(RenderEntity, &Lighting2dSettings, &AmbientLight2d), With<Camera2d>>,
    >,
) {
    for (e, settings, ambient_light) in &ambient_light_query {
        let linear = ambient_light.color.to_linear() * ambient_light.intensity;
        commands.entity(e).insert(ExtractedLighting2dSettings {
            scale: settings.scale,
            ambient_light: linear,
            raymarch: settings.raymarch.clone(),
            penetration: settings.penetration.clone(),
            tint_occluders: if settings.tint_occluders { 1 } else { 0 },
            edge_intensity: settings.edge_intensity,
            blur: settings.blur as i32,
        });
    }
}

#[derive(Component, Deref)]
pub struct Lighting2dCompositePipelineId(pub CachedRenderPipelineId);

pub fn prepare_composite_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut composite_pipelines: ResMut<SpecializedRenderPipelines<Lighting2dCompositePipeline>>,
    composite_pipeline: Res<Lighting2dCompositePipeline>,
    views_query: Query<(Entity, &ExtractedView, &Msaa), With<ExtractedLighting2dSettings>>,
) {
    for (entity, view, msaa) in &views_query {
        commands
            .entity(entity)
            .insert(Lighting2dCompositePipelineId(
                composite_pipelines.specialize(
                    &pipeline_cache,
                    &composite_pipeline,
                    Lighting2dPipelineKey {
                        hdr: view.hdr,
                        msaa_samples: msaa.samples(),
                    },
                ),
            ));
    }
}
