use bevy::{
    core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    ecs::{query::QueryItem, system::lifetimeless::Read},
    prelude::*,
    render::{
        extract_component::DynamicUniformIndex,
        render_asset::RenderAssets,
        render_graph::{NodeRunError, RenderGraphContext, RenderLabel, ViewNode},
        render_resource::{
            binding_types::{sampler, storage_buffer_read_only, texture_2d, uniform_buffer},
            BindGroupLayout, BindGroupLayoutEntries, CachedRenderPipelineId, ColorTargetState,
            ColorWrites, FragmentState, IntoBindGroupLayoutEntryBuilderArray, PipelineCache,
            RenderPipelineDescriptor, SamplerBindingType, ShaderStages, SpecializedRenderPipeline,
            TextureFormat, TextureSampleType,
        },
        renderer::{RenderContext, RenderDevice},
        texture::GpuImage,
        view::{ViewTarget, ViewUniform, ViewUniformOffset},
    },
};

use crate::{
    extract::{ExtractedLightOccluder2d, ExtractedLighting2dSettings, ExtractedPointLight2d},
    passes::{BlurPass, LightingPass, PostProcessPass, SdfPass},
    prepare::Lighting2dTextures,
    queue::Lighting2dPostProcessPipelineId,
    sdf::SdfMaskBindGroup,
};

pub const TYPES_SHADER: Handle<Shader> = Handle::weak_from_u128(76578417911493);
pub const VIEW_TRANSFORMATIONS_SHADER: Handle<Shader> = Handle::weak_from_u128(43290875047924);
pub const FLOOD_INIT_SHADER: Handle<Shader> = Handle::weak_from_u128(32132157492758);
pub const SDF_SHADER: Handle<Shader> = Handle::weak_from_u128(57492774892945);
pub const LIGHTING_SHADER: Handle<Shader> = Handle::weak_from_u128(47320975447604);
pub const BLUR_SHADER: Handle<Shader> = Handle::weak_from_u128(43806754295913);
pub const POST_PROCESS_SHADER: Handle<Shader> = Handle::weak_from_u128(57420546547174);

fn create_pipeline<const N: usize>(
    render_device: &RenderDevice,
    pipeline_cache: &PipelineCache,
    label: &'static str,
    shader: Handle<Shader>,
    layout: impl IntoBindGroupLayoutEntryBuilderArray<N>,
) -> (BindGroupLayout, CachedRenderPipelineId) {
    let layout = render_device.create_bind_group_layout(
        &(String::from(label) + "bind_group_layout") as &str,
        &BindGroupLayoutEntries::sequential(ShaderStages::FRAGMENT, layout),
    );

    let pipeline = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some((String::from(label) + "pipeline").into()),
        layout: vec![layout.clone()],
        vertex: fullscreen_shader_vertex_state(),
        fragment: Some(FragmentState {
            shader,
            shader_defs: vec![],
            entry_point: "fragment".into(),
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

#[derive(Resource)]
pub struct Lighting2dPrepassPipelines {
    pub sdf_layout: BindGroupLayout,
    pub sdf_pipeline: CachedRenderPipelineId,
    pub lighting_layout: BindGroupLayout,
    pub lighting_pipeline: CachedRenderPipelineId,
    pub blur_layout: BindGroupLayout,
    pub blur_pipeline: CachedRenderPipelineId,
}

impl FromWorld for Lighting2dPrepassPipelines {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let (sdf_layout, sdf_pipeline) = create_pipeline(
            render_device,
            pipeline_cache,
            "sdf",
            SDF_SHADER,
            (
                uniform_buffer::<ViewUniform>(true),
                storage_buffer_read_only::<ExtractedLightOccluder2d>(false),
                uniform_buffer::<u32>(false),
            ),
        );

        let (lighting_layout, lighting_pipeline) = create_pipeline(
            render_device,
            pipeline_cache,
            "lighting",
            LIGHTING_SHADER,
            (
                uniform_buffer::<ViewUniform>(true),
                uniform_buffer::<ExtractedLighting2dSettings>(true),
                storage_buffer_read_only::<ExtractedPointLight2d>(false),
                uniform_buffer::<u32>(false),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
            ),
        );

        let (blur_layout, blur_pipeline) = create_pipeline(
            render_device,
            pipeline_cache,
            "blur",
            BLUR_SHADER,
            (
                uniform_buffer::<ViewUniform>(true),
                uniform_buffer::<ExtractedLighting2dSettings>(true),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
            ),
        );

        Self {
            sdf_layout,
            sdf_pipeline,
            lighting_layout,
            lighting_pipeline,
            blur_layout,
            blur_pipeline,
        }
    }
}

#[derive(Resource)]
pub struct PostProcessPipeline {
    pub layout: BindGroupLayout,
}

impl FromWorld for PostProcessPipeline {
    fn from_world(world: &mut World) -> Self {
        Self {
            layout: world.resource::<RenderDevice>().create_bind_group_layout(
                "post_process_bind_group_layout",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::FRAGMENT,
                    (
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                    ),
                ),
            ),
        }
    }
}

#[derive(Eq, PartialEq, Hash, Clone, Copy)]
pub struct Lighting2dPipelineKey {
    pub hdr: bool,
}

impl SpecializedRenderPipeline for PostProcessPipeline {
    type Key = Lighting2dPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("post_process_pipeline".into()),
            layout: vec![self.layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: POST_PROCESS_SHADER,
                shader_defs: vec![],
                entry_point: "fragment".into(),
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

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct LightingLabel;

#[derive(Default)]
pub struct LightingNode;

impl ViewNode for LightingNode {
    type ViewQuery = (
        Read<ViewTarget>,
        Read<ViewUniformOffset>,
        Read<Lighting2dPostProcessPipelineId>,
        Read<SdfMaskBindGroup>,
        Read<Lighting2dTextures>,
        Read<DynamicUniformIndex<ExtractedLighting2dSettings>>,
        Read<ExtractedLighting2dSettings>,
    );

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        ctx: &mut RenderContext<'w>,
        (
            view_target,
            view_uniform_offset,
            post_process_pipeline_id,
            sdf_mask_bind_group,
            flood_textures,
            settings_uniform_index,
            lighting_settings,
        ): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let Some(mask) = world
            .resource::<RenderAssets<GpuImage>>()
            .get(&sdf_mask_bind_group.handle)
        else {
            return Ok(());
        };

        let mut flood_textures = flood_textures.clone();

        // SDF
        let mut sdf_pass = SdfPass::new(world);
        sdf_pass.execute(
            ctx,
            flood_textures.output(),
            &graph.view_entity(),
            view_uniform_offset.offset,
        );
        flood_textures.flip();

        // Lighting
        let mut lighting_pass = LightingPass::new(world);
        lighting_pass.execute(
            ctx,
            flood_textures.input(),
            flood_textures.output(),
            &graph.view_entity(),
            view_uniform_offset.offset,
            settings_uniform_index.index(),
        );
        flood_textures.flip();

        // Blur
        if lighting_settings.blur > 0.0 {
            let mut blur_pass = BlurPass::new(world);
            blur_pass.execute(
                ctx,
                flood_textures.input(),
                flood_textures.output(),
                view_uniform_offset.offset,
                settings_uniform_index.index(),
            );
            flood_textures.flip();
        }

        // Post Process
        let mut post_process_pass = PostProcessPass::new(world);
        post_process_pass.execute(
            ctx,
            flood_textures.input(),
            view_target,
            post_process_pipeline_id.0,
        );

        Ok(())
    }
}
