use bevy::{
    ecs::{query::QueryItem, system::lifetimeless::Read},
    prelude::*,
    render::{
        camera::ExtractedCamera,
        extract_component::{ComponentUniforms, DynamicUniformIndex},
        render_graph::{NodeRunError, RenderGraphContext, RenderLabel, ViewNode},
        render_resource::{
            BindGroupEntries, CachedRenderPipelineId, Operations, PipelineCache,
            RenderPassColorAttachment, RenderPassDescriptor, SamplerDescriptor,
        },
        renderer::RenderContext,
        texture::CachedTexture,
        view::{ViewTarget, ViewUniformOffset, ViewUniforms},
    },
};
use bevy_voronoi::prelude::VoronoiTexture;

use crate::{
    pipeline::{Lighting2dCompositePipeline, Lighting2dPrepassPipelines},
    plugin::{
        ExtractedLighting2dSettings, ExtractedPointLight2d, Lighing2dViewArrayBuffer,
        Lighting2dCompositePipelineId,
    },
};

fn run_lighting_pass<'w>(
    world: &'w World,
    render_context: &mut RenderContext<'w>,
    camera: &ExtractedCamera,
    input: &CachedTexture,
    output: &CachedTexture,
    view_entity: &Entity,
    view_uniform_offset: u32,
    settings_uniform_offset: u32,
) {
    let prepass_pipelines = world.resource::<Lighting2dPrepassPipelines>();
    let pipeline_cache = world.resource::<PipelineCache>();

    let (Some(pipeline), Some(view_uniforms), Some(lighting_settings_uniforms), Some(point_lights)) = (
        pipeline_cache.get_render_pipeline(prepass_pipelines.lighting_pipeline),
        world.resource::<ViewUniforms>().uniforms.binding(),
        world
            .resource::<ComponentUniforms<ExtractedLighting2dSettings>>()
            .binding(),
        world
            .resource::<Lighing2dViewArrayBuffer<ExtractedPointLight2d>>()
            .get(view_entity),
    ) else {
        return;
    };

    let (Some(point_lights), Some(point_lights_count)) =
        (point_lights.data.binding(), point_lights.count.binding())
    else {
        return;
    };

    let sampler = render_context
        .render_device()
        .create_sampler(&SamplerDescriptor::default());

    let bind_group = render_context.render_device().create_bind_group(
        "lighting2d_bind_group",
        &prepass_pipelines.lighting_layout,
        &BindGroupEntries::sequential((
            view_uniforms,
            lighting_settings_uniforms,
            point_lights,
            point_lights_count,
            &input.default_view,
            &sampler,
        )),
    );

    let mut pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("lighting_pass"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: &output.default_view,
            resolve_target: None,
            ops: Operations::default(),
        })],
        ..default()
    });

    if let Some(viewport) = camera.viewport.as_ref() {
        pass.set_camera_viewport(viewport);
    }

    pass.set_render_pipeline(pipeline);
    pass.set_bind_group(
        0,
        &bind_group,
        &[view_uniform_offset, settings_uniform_offset],
    );
    pass.draw(0..3, 0..1);
}

fn run_blur_pass<'w>(
    world: &'w World,
    render_context: &mut RenderContext<'w>,
    input: &CachedTexture,
    output: &CachedTexture,
    view_uniform_offset: u32,
    settings_uniform_offset: u32,
) {
    let prepass_pipelines = world.resource::<Lighting2dPrepassPipelines>();
    let pipeline_cache = world.resource::<PipelineCache>();

    let (Some(pipeline), Some(view_uniforms), Some(lighting_settings_uniforms)) = (
        pipeline_cache.get_render_pipeline(prepass_pipelines.blur_pipeline),
        world.resource::<ViewUniforms>().uniforms.binding(),
        world
            .resource::<ComponentUniforms<ExtractedLighting2dSettings>>()
            .binding(),
    ) else {
        return;
    };

    let sampler = render_context
        .render_device()
        .create_sampler(&SamplerDescriptor::default());

    let bind_group = render_context.render_device().create_bind_group(
        "blur_bind_group",
        &prepass_pipelines.blur_layout,
        &BindGroupEntries::sequential((
            view_uniforms,
            lighting_settings_uniforms,
            &input.default_view,
            &sampler,
        )),
    );

    let mut pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("blur_pass"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: &output.default_view,
            resolve_target: None,
            ops: Operations::default(),
        })],
        ..default()
    });

    pass.set_render_pipeline(pipeline);
    pass.set_bind_group(
        0,
        &bind_group,
        &[view_uniform_offset, settings_uniform_offset],
    );
    pass.draw(0..3, 0..1);
}

pub fn run_composite_pass<'w>(
    world: &'w World,
    render_context: &mut RenderContext<'w>,
    input: &CachedTexture,
    view_target: &ViewTarget,
    pipeline_id: CachedRenderPipelineId,
) {
    let pipeline_cache = world.resource::<PipelineCache>();

    let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_id) else {
        return;
    };

    let post_process = view_target.post_process_write();
    let sampler = render_context
        .render_device()
        .create_sampler(&SamplerDescriptor::default());

    let bind_group = render_context.render_device().create_bind_group(
        "composite_bind_group",
        &world.resource::<Lighting2dCompositePipeline>().layout,
        &BindGroupEntries::sequential((post_process.source, &input.default_view, &sampler)),
    );

    let mut pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("composite_pass"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: post_process.destination,
            resolve_target: None,
            ops: Operations::default(),
        })],
        ..default()
    });

    pass.set_render_pipeline(pipeline);
    pass.set_bind_group(0, &bind_group, &[]);
    pass.draw(0..3, 0..1);
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
        render_context: &mut RenderContext<'w>,
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

        run_lighting_pass(
            world,
            render_context,
            camera,
            voronoi_texture.input(),
            voronoi_texture.output(),
            &graph.view_entity(),
            view_uniform_offset.offset,
            settings_uniform_index.index(),
        );
        voronoi_texture.flip();

        if lighting_settings.blur > 0.0 {
            run_blur_pass(
                world,
                render_context,
                voronoi_texture.input(),
                voronoi_texture.output(),
                view_uniform_offset.offset,
                settings_uniform_index.index(),
            );
            voronoi_texture.flip();
        }

        run_composite_pass(
            world,
            render_context,
            voronoi_texture.input(),
            view_target,
            composite_pipeline_id.0,
        );

        Ok(())
    }
}
