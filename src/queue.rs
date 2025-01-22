use bevy::{
    prelude::*,
    render::{
        render_resource::{
            CachedRenderPipelineId, PipelineCache, ShaderType, SpecializedRenderPipelines,
        },
        view::{ExtractedView, RenderVisibleEntities},
    },
};

use crate::{
    extract::ExtractedLighting2dSettings,
    pipeline::{Lighting2dPipelineKey, PostProcessPipeline},
    types::{LightOccluder2d, PointLight2d},
};

pub type WithPointLight2d = With<PointLight2d>;
pub type WithLightOccluder2d = With<LightOccluder2d>;

#[derive(Component, ShaderType, Default, Clone)]
pub struct LightOccluder2dBufferSize {
    pub size: u32,
    _padding: UVec3,
}

#[derive(Component, ShaderType, Default, Clone)]
pub struct PointLight2dBufferSize {
    pub size: u32,
    _padding: UVec3,
}

pub fn queue_array_buffer_component_sizes(
    mut commands: Commands,
    view_query: Query<(Entity, &RenderVisibleEntities), With<ExtractedLighting2dSettings>>,
) {
    for (entity, visible_entities) in &view_query {
        commands.entity(entity).insert((
            LightOccluder2dBufferSize {
                size: visible_entities.iter::<With<LightOccluder2d>>().count() as u32,
                _padding: UVec3::ZERO,
            },
            PointLight2dBufferSize {
                size: visible_entities.iter::<With<PointLight2d>>().count() as u32,
                _padding: UVec3::ZERO,
            },
        ));
    }
}

#[derive(Component)]
pub struct Lighting2dPostProcessPipelineId(pub CachedRenderPipelineId);

pub fn queue_post_process_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut post_process_pipelines: ResMut<SpecializedRenderPipelines<PostProcessPipeline>>,
    post_process_pipeline: Res<PostProcessPipeline>,
    views_query: Query<(Entity, &ExtractedView), With<ExtractedLighting2dSettings>>,
) {
    for (entity, view) in &views_query {
        commands
            .entity(entity)
            .insert(Lighting2dPostProcessPipelineId(
                post_process_pipelines.specialize(
                    &pipeline_cache,
                    &post_process_pipeline,
                    Lighting2dPipelineKey { hdr: view.hdr },
                ),
            ));
    }
}
