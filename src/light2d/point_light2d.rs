use bevy::{
    asset::{embedded_asset, embedded_path, AssetPath},
    camera::visibility::{add_visibility_class, VisibilityClass},
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_resource::{AsBindGroup, AsBindGroupShaderType, ShaderType},
        sync_world::SyncToRenderWorld,
        texture::GpuImage,
    },
    shader::ShaderRef,
};

use crate::light2d::render::{CustomLight2dPlugin, Light2dMaterial, Light2dSize};

pub struct PointLight2dPlugin;
impl Plugin for PointLight2dPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "point_light2d.wgsl");
        app.add_plugins(CustomLight2dPlugin::<PointLight2d>::default());
    }
}

/// Represents a point light in a 2D environment
#[derive(Component, Clone, Reflect, AsBindGroup)]
#[require(SyncToRenderWorld, Transform, Visibility, VisibilityClass)]
#[component(on_add = add_visibility_class::<PointLight2d>)]
#[uniform(0, PointLight2dGpuType)]
pub struct PointLight2d {
    /// The color of the point light
    pub color: Color,
    /// The intensity of the point light
    pub intensity: f32,
    /// The radius of the point light not affected by the falloff
    pub inner_radius: f32,
    /// The radius of the point light affected by the falloff
    pub outer_radius: f32,
    /// The radial falloff rate of the point light
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

#[derive(ShaderType)]
pub struct PointLight2dGpuType {
    color: LinearRgba,
    inner_radius: f32,
    outer_radius: f32,
    falloff: f32,
    shadows_enabled: u32,
}

impl AsBindGroupShaderType<PointLight2dGpuType> for PointLight2d {
    fn as_bind_group_shader_type(&self, _images: &RenderAssets<GpuImage>) -> PointLight2dGpuType {
        PointLight2dGpuType {
            color: self.color.to_linear() * self.intensity,
            inner_radius: self.inner_radius,
            outer_radius: self.outer_radius,
            falloff: self.falloff,
            shadows_enabled: if self.shadows_enabled { 1 } else { 0 },
        }
    }
}

impl Light2dMaterial for PointLight2d {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("point_light2d.wgsl")).with_source("embedded"),
        )
    }

    #[inline]
    fn light_size(&self) -> Light2dSize {
        Light2dSize::Explicit(Vec2::splat(self.outer_radius * 2.0))
    }
}
