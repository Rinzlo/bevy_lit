use bevy::{
    ecs::{query::QueryItem, system::lifetimeless::Read},
    prelude::*,
    render::{
        camera::ExtractedCamera,
        render_graph::{NodeRunError, RenderGraphContext, ViewNode},
        render_phase::ViewSortedRenderPhases,
        render_resource::RenderPassDescriptor,
        renderer::RenderContext,
        view::ExtractedView,
    },
};

use crate::render::{Shadow2dPhase, ShadowTextures};

#[derive(Default)]
pub struct Shadow2dDrawNode;
impl ViewNode for Shadow2dDrawNode {
    type ViewQuery = (Read<ExtractedCamera>, Read<ExtractedView>);

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (camera, view): QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.view_entity();

        let Some(shadow_phase) = world
            .resource::<ViewSortedRenderPhases<Shadow2dPhase>>()
            .get(&view.retained_view_entity)
        else {
            return Ok(());
        };

        if shadow_phase.items.is_empty() {
            return Ok(());
        }

        let Some(shadow_attatchment) = world
            .resource::<ShadowTextures>()
            .get(&view.retained_view_entity)
            .map(|t| t.clone())
        else {
            return Ok(());
        };

        let mut pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("shadow2d_pass"),
            color_attachments: &[Some(shadow_attatchment.get_attachment())],
            ..default()
        });

        if let Some(viewport) = camera.viewport.as_ref() {
            pass.set_camera_viewport(viewport);
        }
        if let Err(err) = shadow_phase.render(&mut pass, world, view_entity) {
            error!("Error encountered while rendering the shadow phase {err:?}");
        }

        Ok(())
    }
}
