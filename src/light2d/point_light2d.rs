use bevy::{
    asset::{embedded_asset, embedded_path, AssetPath},
    camera::visibility::{add_visibility_class, VisibilityClass},
    prelude::*,
    render::{
        render_resource::{
            binding_types::uniform_buffer, BindGroup, BindGroupEntries, BindGroupLayout,
            BindGroupLayoutEntries, ShaderStages, ShaderType, UniformBuffer,
        },
        renderer::{RenderDevice, RenderQueue},
        sync_world::SyncToRenderWorld,
    },
    shader::ShaderRef,
};

use crate::light2d::render::{CustomLight2dPlugin, Light2dMaterial};

pub struct PointLight2dPlugin;
impl Plugin for PointLight2dPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "point_light2d.wgsl");
        app.add_plugins(CustomLight2dPlugin::<PointLight2d>::default());
    }
}

/// Represents a point light in a 2D environment
#[derive(Component, Clone, Reflect)]
#[require(SyncToRenderWorld, Transform, Visibility, VisibilityClass)]
#[component(on_add = add_visibility_class::<PointLight2d>)]
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

impl Light2dMaterial for PointLight2d {
    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(
            "point_light2d_layout",
            &BindGroupLayoutEntries::single(
                ShaderStages::FRAGMENT,
                uniform_buffer::<PointLight2dGpuType>(false),
            ),
        )
    }

    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("point_light2d.wgsl")).with_source("embedded"),
        )
    }

    #[inline]
    fn light_size(&self) -> Vec2 {
        Vec2::splat(self.outer_radius * 2.0)
    }

    fn bind_group(&self, render_device: &RenderDevice, render_queue: &RenderQueue) -> BindGroup {
        let mut buffer = UniformBuffer::from(PointLight2dGpuType {
            color: self.color.to_linear() * self.intensity,
            inner_radius: self.inner_radius,
            outer_radius: self.outer_radius,
            falloff: self.falloff,
            shadows_enabled: if self.shadows_enabled { 1 } else { 0 },
        });

        buffer.write_buffer(&render_device, &render_queue);

        render_device.create_bind_group(
            "point_light2d_bind_group",
            &Self::bind_group_layout(render_device),
            &BindGroupEntries::single(buffer.binding().unwrap()),
        )
    }
}
