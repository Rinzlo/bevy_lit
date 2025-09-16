use bevy::{
    asset::embedded_asset,
    core_pipeline::core_2d::{
        extract_core_2d_camera_phases,
        graph::{Core2d, Node2d},
    },
    prelude::*,
    render::{
        render_graph::{RenderGraphExt, ViewNodeRunner},
        render_phase::{AddRenderCommand, DrawFunctions, ViewSortedRenderPhases},
        render_resource::SpecializedRenderPipelines,
        Render, RenderApp, RenderStartup, RenderSystems,
    },
};

use crate::lighting_2d::{
    node::{
        extract_light_2d_phases, CompositeDrawNode, CompositeDrawPassLabel, Light2dDrawNode,
        Light2dDrawPassLabel, Light2dPhase,
    },
    render::{
        extract_light2d_instances, init_light2d_pipeline, prepare_light2d_buffers,
        prepare_light2d_view_bind_groups, prepare_lighting_textures, queue_light2d_instances,
        DrawLight2dMesh, Light2dBatches, Light2dMaterialBindGroups, Light2dMeta, Light2dPipeline,
        LightingTextures, RenderLights2dInstances,
    },
};

pub mod light_2d;
mod node;
mod render;

pub struct Light2dPlugin;
impl Plugin for Light2dPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "light_2d.wgsl");

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<ViewSortedRenderPhases<Light2dPhase>>()
            .init_resource::<RenderLights2dInstances>()
            .init_resource::<DrawFunctions<Light2dPhase>>()
            .init_resource::<SpecializedRenderPipelines<Light2dPipeline>>()
            .init_resource::<Light2dMeta>()
            .init_resource::<Light2dBatches>()
            .init_resource::<Light2dMaterialBindGroups>()
            .init_resource::<LightingTextures>()
            .add_systems(RenderStartup, init_light2d_pipeline)
            .add_systems(
                ExtractSchedule,
                (
                    extract_light_2d_phases.after(extract_core_2d_camera_phases),
                    extract_light2d_instances,
                ),
            )
            .add_systems(
                Render,
                (
                    queue_light2d_instances.in_set(RenderSystems::Queue),
                    (
                        prepare_lighting_textures,
                        prepare_light2d_view_bind_groups,
                        prepare_light2d_buffers,
                    )
                        .in_set(RenderSystems::PrepareBindGroups),
                ),
            )
            .add_render_command::<Light2dPhase, DrawLight2dMesh>()
            .add_render_graph_node::<ViewNodeRunner<Light2dDrawNode>>(Core2d, Light2dDrawPassLabel)
            .add_render_graph_node::<ViewNodeRunner<CompositeDrawNode>>(
                Core2d,
                CompositeDrawPassLabel,
            )
            .add_render_graph_edges(
                Core2d,
                (
                    Node2d::EndMainPass,
                    Light2dDrawPassLabel,
                    CompositeDrawPassLabel,
                ),
            );
    }
}
