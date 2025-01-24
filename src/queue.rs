use bevy::{
    prelude::*,
    render::{
        render_resource::{CachedRenderPipelineId, PipelineCache, SpecializedRenderPipelines},
        view::ExtractedView,
    },
};

use crate::{
    extract::ExtractedLighting2dSettings,
    pipeline::{Lighting2dPipelineKey, PostProcessPipeline},
};

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
