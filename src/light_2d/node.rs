use std::ops::Range;

use bevy::{
    ecs::{query::QueryItem, system::lifetimeless::Read},
    math::FloatOrd,
    platform::collections::HashSet,
    prelude::*,
    render::{
        camera::ExtractedCamera,
        render_graph::{NodeRunError, RenderGraphContext, RenderLabel, ViewNode},
        render_phase::{
            CachedRenderPipelinePhaseItem, DrawFunctionId, PhaseItem, PhaseItemExtraIndex,
            SortedPhaseItem, ViewSortedRenderPhases,
        },
        render_resource::{
            CachedRenderPipelineId, Operations, RenderPassColorAttachment, RenderPassDescriptor,
        },
        renderer::RenderContext,
        sync_world::MainEntity,
        view::{ExtractedView, RetainedViewEntity},
        Extract,
    },
};

use crate::{
    light_2d::render::LightingTextures, post_process::lighting_settings_2d::Lighting2dSettings,
};

pub struct Light2dPhase {
    pub sort_key: FloatOrd,
    pub pipeline: CachedRenderPipelineId,
    pub draw_function: DrawFunctionId,
    pub entity: (Entity, MainEntity),
    pub batch_range: Range<u32>,
    pub extra_index: PhaseItemExtraIndex,
    pub indexed: bool,
}

impl PhaseItem for Light2dPhase {
    #[inline]
    fn entity(&self) -> Entity {
        self.entity.0
    }

    #[inline]
    fn main_entity(&self) -> MainEntity {
        self.entity.1
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn batch_range(&self) -> &Range<u32> {
        &self.batch_range
    }

    #[inline]
    fn batch_range_mut(&mut self) -> &mut Range<u32> {
        &mut self.batch_range
    }

    #[inline]
    fn extra_index(&self) -> PhaseItemExtraIndex {
        self.extra_index.clone()
    }

    #[inline]
    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl SortedPhaseItem for Light2dPhase {
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        self.sort_key
    }

    #[inline]
    fn indexed(&self) -> bool {
        self.indexed
    }
}

impl CachedRenderPipelinePhaseItem for Light2dPhase {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

pub fn extract_light_2d_phases(
    cameras: Extract<Query<(Entity, &Camera), (With<Camera2d>, With<Lighting2dSettings>)>>,
    mut light2d_phases: ResMut<ViewSortedRenderPhases<Light2dPhase>>,
    mut live_entities: Local<HashSet<RetainedViewEntity>>,
) {
    live_entities.clear();

    for (entity, camera) in &cameras {
        if !camera.is_active {
            continue;
        }

        let retained_view_entity = RetainedViewEntity::new(entity.into(), None, 0);

        light2d_phases.insert_or_clear(retained_view_entity);
        live_entities.insert(retained_view_entity);
    }

    // Clear out all dead views
    light2d_phases.retain(|camera_entity, _| live_entities.contains(camera_entity));
}

#[derive(RenderLabel, Debug, Clone, Hash, PartialEq, Eq)]
pub struct Light2dDrawPassLabel;

#[derive(Default)]
pub struct Light2dDrawNode;
impl ViewNode for Light2dDrawNode {
    type ViewQuery = (Read<ExtractedCamera>, Read<ExtractedView>);

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (camera, view): QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.view_entity();

        let Some(light_phase) = world
            .resource::<ViewSortedRenderPhases<Light2dPhase>>()
            .get(&view.retained_view_entity)
        else {
            return Ok(());
        };

        if light_phase.items.is_empty() {
            return Ok(());
        }

        let mut lighting_texture = world
            .resource::<LightingTextures>()
            .get(&view.retained_view_entity)
            .expect(&format!(
                "Expected the lighting texture for view {:?} to exist",
                view.retained_view_entity.main_entity.id()
            ))
            .clone();

        let mut pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("light2d_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &lighting_texture.input().default_view,
                resolve_target: None,
                ops: Operations::default(),
                depth_slice: None,
            })],
            ..default()
        });

        if let Some(viewport) = camera.viewport.as_ref() {
            pass.set_camera_viewport(viewport);
        }

        if let Err(err) = light_phase.render(&mut pass, world, view_entity) {
            error!("Error encountered while rendering the lighting phase {err:?}");
        }

        lighting_texture.flip();

        Ok(())
    }
}
