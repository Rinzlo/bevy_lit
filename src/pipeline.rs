use bevy::{
    core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    ecs::{query::QueryItem, system::lifetimeless::Read},
    prelude::*,
    render::{
        camera::ExtractedCamera,
        extract_component::DynamicUniformIndex,
        render_graph::{NodeRunError, RenderGraphContext, RenderLabel, ViewNode},
        render_resource::{
            binding_types::{sampler, storage_buffer_read_only, texture_2d, uniform_buffer},
            BindGroupLayout, BindGroupLayoutEntries, BindGroupLayoutEntry, CachedRenderPipelineId,
            ColorTargetState, ColorWrites, FragmentState, PipelineCache, RenderPipelineDescriptor,
            SamplerBindingType, ShaderStages, SpecializedRenderPipeline, TextureFormat,
            TextureSampleType,
        },
        renderer::{RenderContext, RenderDevice},
        view::{ViewTarget, ViewUniform, ViewUniformOffset},
    },
};

use flood_plugin::prelude::VoronoiTexture;

use crate::{
    extract::{ExtractedLighting2dSettings, ExtractedPointLight2d},
    passes::{BlurPass, CompositePass, LightingPass},
    queue::Lighting2dCompositePipelineId,
};

pub const TYPES_SHADER: Handle<Shader> = Handle::weak_from_u128(76578417911493);
pub const VIEW_TRANSFORMATIONS_SHADER: Handle<Shader> = Handle::weak_from_u128(43290875047924);
pub const LIGHTING_SHADER: Handle<Shader> = Handle::weak_from_u128(47320975447604);
pub const BLUR_SHADER: Handle<Shader> = Handle::weak_from_u128(43806754295913);
pub const COMPOSITE_SHADER: Handle<Shader> = Handle::weak_from_u128(57420546547174);

fn create_pipeline(
    render_device: &RenderDevice,
    pipeline_cache: &PipelineCache,
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
    pub lighting_layout: BindGroupLayout,
    pub lighting_pipeline: CachedRenderPipelineId,
    pub blur_layout: BindGroupLayout,
    pub blur_pipeline: CachedRenderPipelineId,
}

impl FromWorld for Lighting2dPrepassPipelines {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let (lighting_layout, lighting_pipeline) = create_pipeline(
            render_device,
            pipeline_cache,
            "lighting",
            LIGHTING_SHADER,
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    uniform_buffer::<ViewUniform>(true),
                    uniform_buffer::<ExtractedLighting2dSettings>(true),
                    storage_buffer_read_only::<ExtractedPointLight2d>(false),
                    uniform_buffer::<u32>(false),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );

        let (blur_layout, blur_pipeline) = create_pipeline(
            render_device,
            pipeline_cache,
            "blur",
            BLUR_SHADER,
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

        Self {
            lighting_layout,
            lighting_pipeline,
            blur_layout,
            blur_pipeline,
        }
    }
}

#[derive(Resource)]
pub struct Lighting2dCompositePipeline {
    pub layout: BindGroupLayout,
}

impl FromWorld for Lighting2dCompositePipeline {
    fn from_world(world: &mut World) -> Self {
        Self {
            layout: world.resource::<RenderDevice>().create_bind_group_layout(
                "composite_bind_group_layout",
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

impl SpecializedRenderPipeline for Lighting2dCompositePipeline {
    type Key = Lighting2dPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("composite_pipeline".into()),
            layout: vec![self.layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: COMPOSITE_SHADER,
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
        Read<ExtractedCamera>,
        Read<ViewUniformOffset>,
        Read<Lighting2dCompositePipelineId>,
        Read<VoronoiTexture>,
        Read<DynamicUniformIndex<ExtractedLighting2dSettings>>,
        Read<ExtractedLighting2dSettings>,
    );

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        ctx: &mut RenderContext<'w>,
        (
            view_target,
            camera,
            view_uniform_offset,
            composite_pipeline_id,
            voronoi_texture,
            settings_uniform_index,
            lighting_settings,
        ): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let mut voronoi_texture = voronoi_texture.clone();

        // Lighting
        let mut lighting_pass = LightingPass::new(world);
        lighting_pass.execute(
            ctx,
            camera,
            voronoi_texture.input(),
            voronoi_texture.output(),
            &graph.view_entity(),
            view_uniform_offset.offset,
            settings_uniform_index.index(),
        );
        voronoi_texture.flip();

        // Blur
        if lighting_settings.blur > 0.0 {
            let mut blur_pass = BlurPass::new(world);
            blur_pass.execute(
                ctx,
                voronoi_texture.input(),
                voronoi_texture.output(),
                view_uniform_offset.offset,
                settings_uniform_index.index(),
            );
            voronoi_texture.flip();
        }

        // Post Process
        let mut composite_pass = CompositePass::new(world);
        composite_pass.execute(
            ctx,
            voronoi_texture.input(),
            view_target,
            composite_pipeline_id.0,
        );

        Ok(())
    }
}
