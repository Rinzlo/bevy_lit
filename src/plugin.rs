use bevy::{prelude::*, shader::load_shader_library};

use crate::{
    light2d::{point_light2d::PointLight2d, render::Light2dPlugin, spot_light2d::SpotLight2d},
    post_process::Lighting2dSettingsPlugin,
    render::Light2dRenderPlugin,
    shadows2d::Shadows2dPlugin,
};

/// A plugin for adding 2D lighting in the Bevy engine.
///
/// This plugin sets up and configures the necessary components and systems for 2D lighting,
/// including [`AmbientLight2d`], [`Lighting2dSettings`], [`PointLight2d`], and [`LightOccluder2d`].
pub struct Lighting2dPlugin;

impl Plugin for Lighting2dPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "view_transformations.wgsl");
        load_shader_library!(app, "settings_types.wgsl");

        app.add_plugins((
            Light2dRenderPlugin,
            Lighting2dSettingsPlugin,
            Shadows2dPlugin,
            Light2dPlugin::<PointLight2d>::default(),
            Light2dPlugin::<SpotLight2d>::default(),
        ));
    }
}
