use bevy::{
    ecs::{query::QueryItem, system::lifetimeless::Read},
    prelude::*,
    render::{
        camera::ExtractedCamera,
        render_graph::{NodeRunError, RenderGraphContext, ViewNode},
        render_phase::ViewSortedRenderPhases,
        render_resource::{Operations, RenderPassColorAttachment, RenderPassDescriptor},
        renderer::RenderContext,
        view::ExtractedView,
    },
};

use crate::render::{Light2dPhase, LightingTextures};

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

        let Some(mut lighting_texture) = world
            .resource::<LightingTextures>()
            .get(&view.retained_view_entity)
            .cloned()
        else {
            return Ok(());
        };

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
