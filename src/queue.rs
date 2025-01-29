use bevy::{
    prelude::*,
    render::{
        render_resource::{CachedRenderPipelineId, PipelineCache, SpecializedRenderPipelines},
        view::ExtractedView,
    },
};

use crate::{
    extract::ExtractedLighting2dSettings,
    pipeline::{Lighting2dCompositePipeline, Lighting2dPipelineKey},
};

#[derive(Component)]
pub struct Lighting2dCompositePipelineId(pub CachedRenderPipelineId);

pub fn queue_composite_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut composite_pipelines: ResMut<SpecializedRenderPipelines<Lighting2dCompositePipeline>>,
    composite_pipeline: Res<Lighting2dCompositePipeline>,
    views_query: Query<(Entity, &ExtractedView), With<ExtractedLighting2dSettings>>,
) {
    for (entity, view) in &views_query {
        commands
            .entity(entity)
            .insert(Lighting2dCompositePipelineId(
                composite_pipelines.specialize(
                    &pipeline_cache,
                    &composite_pipeline,
                    Lighting2dPipelineKey { hdr: view.hdr },
                ),
            ));
    }
}
