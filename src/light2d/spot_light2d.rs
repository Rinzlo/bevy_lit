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

pub struct SpotLight2dPlugin;
impl Plugin for SpotLight2dPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "spot_light2d.wgsl");
        app.add_plugins(CustomLight2dPlugin::<SpotLight2d>::default());
    }
}

/// Represents a spot light in a 2D environment
#[derive(Component, Clone, Reflect)]
#[require(SyncToRenderWorld, Transform, Visibility, VisibilityClass)]
#[component(on_add = add_visibility_class::<SpotLight2d>)]
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
    /// The angle of the spot light affected by the angular falloff
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

#[derive(ShaderType)]
pub struct SpotLight2dGpuType {
    color: LinearRgba,
    inner_radius: f32,
    outer_radius: f32,
    radial_falloff: f32,
    inner_angle: f32,
    outer_angle: f32,
    angular_falloff: f32,
    shadows_enabled: u32,
}

impl Light2dMaterial for SpotLight2d {
    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(
            "spot_light2d_layout",
            &BindGroupLayoutEntries::single(
                ShaderStages::FRAGMENT,
                uniform_buffer::<SpotLight2dGpuType>(false),
            ),
        )
    }

    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("spot_light2d.wgsl")).with_source("embedded"),
        )
    }

    #[inline]
    fn light_size(&self) -> Vec2 {
        Vec2::splat(self.outer_radius * 2.0)
    }

    fn bind_group(&self, render_device: &RenderDevice, render_queue: &RenderQueue) -> BindGroup {
        let mut buffer = UniformBuffer::from(SpotLight2dGpuType {
            color: self.color.to_linear() * self.intensity,
            inner_radius: self.inner_radius,
            outer_radius: self.outer_radius,
            radial_falloff: self.radial_falloff,
            inner_angle: self.inner_angle.to_radians(),
            outer_angle: self.outer_angle.to_radians(),
            angular_falloff: self.angular_falloff,
            shadows_enabled: if self.shadows_enabled { 1 } else { 0 },
        });

        buffer.write_buffer(&render_device, &render_queue);

        render_device.create_bind_group(
            "spot_light2d_bind_group",
            &Self::bind_group_layout(render_device),
            &BindGroupEntries::single(buffer.binding().unwrap()),
        )
    }
}
