use bevy::{
    prelude::*,
    render::{
        extract_component::ComponentUniforms,
        render_resource::{
            BindGroup, BindGroupEntries, GpuArrayBuffer, SamplerDescriptor, TextureDescriptor,
            TextureDimension, TextureFormat, TextureUsages,
        },
        renderer::RenderDevice,
        texture::{CachedTexture, TextureCache},
        view::{ViewTarget, ViewUniforms},
    },
};

use crate::{
    extract::{ExtractedLightOccluder2d, ExtractedLighting2dSettings, ExtractedPointLight2d},
    pipeline::Lighting2dPrepassPipelines,
    queue::{LightOccluder2dBufferCount, PointLight2dBufferCount},
};

fn create_aux_texture(
    view_target: &ViewTarget,
    texture_cache: &mut TextureCache,
    render_device: &RenderDevice,
    label: &'static str,
) -> CachedTexture {
    texture_cache.get(
        render_device,
        TextureDescriptor {
            label: Some(label),
            size: view_target.main_texture().size(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba16Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
    )
}

#[derive(Component)]
pub struct Lighting2dAuxiliaryTextures {
    pub sdf: CachedTexture,
    pub lighting: CachedTexture,
    pub blur: Option<CachedTexture>,
}

pub fn prepare_lighting_auxiliary_textures(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    view_targets: Query<(Entity, &ViewTarget, &ExtractedLighting2dSettings)>,
) {
    for (entity, view_target, settings) in &view_targets {
        commands.entity(entity).insert(Lighting2dAuxiliaryTextures {
            sdf: create_aux_texture(view_target, &mut texture_cache, &render_device, "sdf"),
            lighting: create_aux_texture(
                view_target,
                &mut texture_cache,
                &render_device,
                "lighting",
            ),
            blur: if settings.blur > 0.0 {
                Some(create_aux_texture(
                    view_target,
                    &mut texture_cache,
                    &render_device,
                    "blur",
                ))
            } else {
                None
            },
        });
    }
}

#[derive(Component)]
pub struct Lighting2dSurfaceBindGroups {
    pub sdf: BindGroup,
    pub lighting: BindGroup,
    pub blur: BindGroup,
}

pub fn prepare_lighting_bind_groups(
    mut commands: Commands,
    prepass_pipelines: Res<Lighting2dPrepassPipelines>,
    render_device: Res<RenderDevice>,
    view_uniforms: Res<ViewUniforms>,
    lighting_settings: Res<ComponentUniforms<ExtractedLighting2dSettings>>,
    light_occluders: Res<GpuArrayBuffer<ExtractedLightOccluder2d>>,
    occluders_buffer_count: Res<ComponentUniforms<LightOccluder2dBufferCount>>,
    point_lights: Res<GpuArrayBuffer<ExtractedPointLight2d>>,
    point_lights_buffer_count: Res<ComponentUniforms<PointLight2dBufferCount>>,
    views_query: Query<(Entity, &Lighting2dAuxiliaryTextures), With<ExtractedLighting2dSettings>>,
) {
    let (
        Some(view_uniforms),
        Some(lighting_settings),
        Some(light_occluders),
        Some(occluders_buffer_count),
        Some(point_lights),
        Some(point_lights_buffer_count),
    ) = (
        view_uniforms.uniforms.binding(),
        lighting_settings.binding(),
        light_occluders.binding(),
        occluders_buffer_count.binding(),
        point_lights.binding(),
        point_lights_buffer_count.binding(),
    )
    else {
        return;
    };

    let sampler = render_device.create_sampler(&SamplerDescriptor::default());

    for (entity, aux_textures) in &views_query {
        commands.entity(entity).insert(Lighting2dSurfaceBindGroups {
            sdf: render_device.create_bind_group(
                "sdf_bind_group",
                &prepass_pipelines.sdf_layout,
                &BindGroupEntries::sequential((
                    view_uniforms.clone(),
                    light_occluders.clone(),
                    occluders_buffer_count.clone(),
                )),
            ),
            lighting: render_device.create_bind_group(
                "lighting2d_bind_group",
                &prepass_pipelines.lighting_layout,
                &BindGroupEntries::sequential((
                    view_uniforms.clone(),
                    lighting_settings.clone(),
                    point_lights.clone(),
                    point_lights_buffer_count.clone(),
                    &aux_textures.sdf.default_view,
                    &sampler,
                )),
            ),
            blur: render_device.create_bind_group(
                "blur_bind_group",
                &prepass_pipelines.blur_layout,
                &BindGroupEntries::sequential((
                    view_uniforms.clone(),
                    lighting_settings.clone(),
                    &aux_textures.lighting.default_view,
                    &sampler,
                )),
            ),
        });
    }
}
