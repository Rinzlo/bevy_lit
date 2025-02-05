use bevy::{
    prelude::*,
    render::{
        camera::ExtractedCamera,
        extract_component::ComponentUniforms,
        render_resource::{
            BindGroupEntries, CachedRenderPipelineId, Operations, PipelineCache,
            RenderPassColorAttachment, RenderPassDescriptor, SamplerDescriptor,
        },
        renderer::RenderContext,
        texture::CachedTexture,
        view::{ViewTarget, ViewUniforms},
    },
};

use crate::{
    extract::{ExtractedLighting2dSettings, ExtractedPointLight2d},
    pipeline::{Lighting2dCompositePipeline, Lighting2dPrepassPipelines},
    prepare::Lighing2dViewArrayBuffer,
};

pub struct LightingPass<'w> {
    world: &'w World,
}

impl<'w> LightingPass<'w> {
    pub fn new(world: &'w World) -> Self {
        Self { world }
    }

    pub fn execute(
        &mut self,
        ctx: &mut RenderContext<'_>,
        camera: &ExtractedCamera,
        input: &CachedTexture,
        output: &CachedTexture,
        view_entity: &Entity,
        view_uniform_offset: u32,
        settings_uniform_offset: u32,
    ) {
        let world = self.world;
        let prepass_pipelines = world.resource::<Lighting2dPrepassPipelines>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let (
            Some(pipeline),
            Some(view_uniforms),
            Some(lighting_settings_uniforms),
            Some(point_lights),
        ) = (
            pipeline_cache.get_render_pipeline(prepass_pipelines.lighting_pipeline),
            world.resource::<ViewUniforms>().uniforms.binding(),
            world
                .resource::<ComponentUniforms<ExtractedLighting2dSettings>>()
                .binding(),
            world
                .resource::<Lighing2dViewArrayBuffer<ExtractedPointLight2d>>()
                .get(view_entity),
        )
        else {
            return;
        };

        let (Some(point_lights), Some(point_lights_count)) =
            (point_lights.data.binding(), point_lights.count.binding())
        else {
            return;
        };

        let sampler = ctx
            .render_device()
            .create_sampler(&SamplerDescriptor::default());

        let bind_group = ctx.render_device().create_bind_group(
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

        let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
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
}

pub struct BlurPass<'w> {
    world: &'w World,
}

impl<'w> BlurPass<'w> {
    pub fn new(world: &'w World) -> Self {
        Self { world }
    }

    pub fn execute(
        &mut self,
        ctx: &mut RenderContext<'_>,
        input: &CachedTexture,
        output: &CachedTexture,
        view_uniform_offset: u32,
        settings_uniform_offset: u32,
    ) {
        let world = self.world;
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

        let sampler = ctx
            .render_device()
            .create_sampler(&SamplerDescriptor::default());

        let bind_group = ctx.render_device().create_bind_group(
            "blur_bind_group",
            &prepass_pipelines.blur_layout,
            &BindGroupEntries::sequential((
                view_uniforms,
                lighting_settings_uniforms,
                &input.default_view,
                &sampler,
            )),
        );

        let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
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
}

pub struct CompositePass<'w> {
    world: &'w World,
}

impl<'w> CompositePass<'w> {
    pub fn new(world: &'w World) -> Self {
        Self { world }
    }

    pub fn execute(
        &mut self,
        ctx: &mut RenderContext<'_>,
        input: &CachedTexture,
        view_target: &ViewTarget,
        pipeline_id: CachedRenderPipelineId,
    ) {
        let pipeline_cache = self.world.resource::<PipelineCache>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_id) else {
            return;
        };

        let post_process = view_target.post_process_write();
        let sampler = ctx
            .render_device()
            .create_sampler(&SamplerDescriptor::default());

        let bind_group = ctx.render_device().create_bind_group(
            "composite_bind_group",
            &self.world.resource::<Lighting2dCompositePipeline>().layout,
            &BindGroupEntries::sequential((post_process.source, &input.default_view, &sampler)),
        );

        let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
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
}
