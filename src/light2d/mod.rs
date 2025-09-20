use bevy::{
    asset::embedded_asset,
    camera::visibility::{add_visibility_class, VisibilityClass},
    prelude::*,
    render::{
        render_phase::AddRenderCommand, render_resource::SpecializedRenderPipelines,
        sync_world::SyncToRenderWorld, Render, RenderApp, RenderStartup, RenderSystems,
    },
    shader::load_shader_library,
};

use crate::{
    light2d::render::{
        extract_light2d_instances, init_light2d_pipeline, prepare_light2d_buffers,
        prepare_light2d_view_bind_groups, queue_light2d_instances, DrawLight2dMesh, Light2dBatches,
        Light2dMaterialBindGroups, Light2dMeta, Light2dPipeline, RenderLights2dInstances,
    },
    render::Light2dPhase,
};

pub mod render;

pub struct Light2dPlugin;
impl Plugin for Light2dPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "light2d_common.wgsl");
        embedded_asset!(app, "light2d.wgsl");
        embedded_asset!(app, "point_light2d.wgsl");
        embedded_asset!(app, "spot_light2d.wgsl");

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<RenderLights2dInstances>()
            .init_resource::<SpecializedRenderPipelines<Light2dPipeline>>()
            .init_resource::<Light2dMeta>()
            .init_resource::<Light2dBatches>()
            .init_resource::<Light2dMaterialBindGroups>()
            .add_systems(ExtractSchedule, extract_light2d_instances)
            .add_systems(RenderStartup, init_light2d_pipeline)
            .add_systems(
                Render,
                (
                    queue_light2d_instances.in_set(RenderSystems::Queue),
                    (prepare_light2d_view_bind_groups, prepare_light2d_buffers)
                        .in_set(RenderSystems::PrepareBindGroups),
                ),
            )
            .add_render_command::<Light2dPhase, DrawLight2dMesh>();
    }
}

#[derive(Clone, Copy)]
pub struct PointLight2d {
    /// The color of the point light
    pub color: Color,
    /// The intensity of the point light
    pub intensity: f32,
    /// The radius of the point light not affected by the falloff
    pub inner_radius: f32,
    /// The radius of the point light affected by the falloff
    pub outer_radius: f32,
    /// The falloff rate of the point light
    pub falloff: f32,
    /// Whether the point light should project shadows
    pub shadows_enabled: bool,
}

impl Default for PointLight2d {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            intensity: 1.0,
            inner_radius: 0.0,
            outer_radius: 64.0,
            falloff: 1.0,
            shadows_enabled: true,
        }
    }
}

pub struct SpotLight2d {
    /// The color of the spot light
    pub color: Color,
    /// The intensity of the spot light
    pub intensity: f32,
    /// The radius of the spot light not affected by the radial falloff
    pub inner_radius: f32,
    /// The radius of the spot light affected by the radial falloff
    pub outer_radius: f32,
    /// The radial falloff rate of the spot light
    pub radial_falloff: f32,
    /// The angle of the spot light not affected by the angular falloff
    pub inner_angle: f32,
    /// The angle of the spot lightaffected by the angular falloff
    pub outer_angle: f32,
    /// The angular falloff rate of the spot light
    pub angular_falloff: f32,
    /// Whether the spot light should project shadows
    pub shadows_enabled: bool,
}

impl Default for SpotLight2d {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            intensity: 1.0,
            inner_radius: 0.0,
            outer_radius: 64.0,
            radial_falloff: 1.0,
            inner_angle: 0.0,
            outer_angle: 45.0,
            angular_falloff: 1.0,
            shadows_enabled: true,
        }
    }
}

#[derive(Component, Clone, Reflect)]
#[require(SyncToRenderWorld, Transform, Visibility, VisibilityClass)]
#[component(on_add = add_visibility_class::<Light2d>)]
pub enum Light2d {
    Point {
        color: Color,
        intensity: f32,
        inner_radius: f32,
        outer_radius: f32,
        falloff: f32,
        shadows_enabled: bool,
    },
    Spot {
        color: Color,
        intensity: f32,
        inner_radius: f32,
        outer_radius: f32,
        radial_falloff: f32,
        inner_angle: f32,
        outer_angle: f32,
        angular_falloff: f32,
        shadows_enabled: bool,
    },
}

impl From<PointLight2d> for Light2d {
    fn from(light: PointLight2d) -> Self {
        Self::Point {
            color: light.color,
            intensity: light.intensity,
            inner_radius: light.inner_radius,
            outer_radius: light.outer_radius,
            falloff: light.falloff,
            shadows_enabled: light.shadows_enabled,
        }
    }
}

impl From<SpotLight2d> for Light2d {
    fn from(light: SpotLight2d) -> Self {
        Self::Spot {
            color: light.color,
            intensity: light.intensity,
            inner_radius: light.inner_radius,
            outer_radius: light.outer_radius,
            radial_falloff: light.radial_falloff,
            inner_angle: light.inner_angle,
            outer_angle: light.outer_angle,
            angular_falloff: light.angular_falloff,
            shadows_enabled: light.shadows_enabled,
        }
    }
}
