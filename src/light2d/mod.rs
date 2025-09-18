use bevy::{
    camera::visibility::{add_visibility_class, VisibilityClass},
    prelude::*,
    render::sync_world::SyncToRenderWorld,
};

pub mod node;
pub mod render;

#[derive(Component, Clone, Reflect)]
#[require(SyncToRenderWorld, Transform, Visibility, VisibilityClass)]
#[component(on_add = add_visibility_class::<Light2d>)]
pub enum Light2d {
    Point {
        /// The color of the point light.
        color: Color,
        /// The intensity of the point light.
        intensity: f32,
        /// The radius of the point light's influence.
        inner_radius: f32,
        outer_radius: f32,
        /// The falloff rate of the point light.
        falloff: f32,
        /// wether the point light should project shadows
        shadows_enabled: bool,
    },
}

impl Default for Light2d {
    fn default() -> Self {
        Self::Point {
            color: Color::WHITE,
            intensity: 1.0,
            inner_radius: 0.0,
            outer_radius: 64.0,
            falloff: 1.0,
            shadows_enabled: true,
        }
    }
}
