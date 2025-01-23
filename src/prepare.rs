use bevy::{
    ecs::entity::{EntityHashMap, EntityHashSet},
    prelude::*,
    render::{
        extract_component::ComponentUniforms,
        render_resource::{
            BindGroup, BindGroupEntries, GpuArrayBufferable, SamplerDescriptor, StorageBuffer,
            TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, UniformBuffer,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::{CachedTexture, TextureCache},
        view::{RenderVisibleEntities, ViewTarget, ViewUniforms},
    },
};

use crate::{
    extract::{ExtractedLightOccluder2d, ExtractedLighting2dSettings, ExtractedPointLight2d},
    pipeline::Lighting2dPrepassPipelines,
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

#[derive(Deref, DerefMut)]
pub struct Lighting2dArrayBuffer<T: GpuArrayBufferable> {
    #[deref]
    pub data: StorageBuffer<Vec<T>>,
    pub count: UniformBuffer<u32>,
}

impl<T: GpuArrayBufferable> Lighting2dArrayBuffer<T> {
    pub fn new(data: Vec<T>, count: u32) -> Self {
        Self {
            data: StorageBuffer::from(data),
            count: UniformBuffer::from(count),
        }
    }

    pub fn write(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        self.data.write_buffer(device, queue);
        self.count.write_buffer(device, queue);
    }
}

#[derive(Resource, Deref, DerefMut, Default)]
pub struct Lighing2dViewArrayBuffer<T: GpuArrayBufferable>(
    pub EntityHashMap<Lighting2dArrayBuffer<T>>,
);

pub fn prepare_lighting2d_view_array_buffers<T: Component + GpuArrayBufferable, U: Component>(
    mut view_entities: Local<EntityHashSet>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut view_array_buffer: ResMut<Lighing2dViewArrayBuffer<T>>,
    view_query: Query<(Entity, &RenderVisibleEntities), With<ExtractedLighting2dSettings>>,
    components: Query<(Entity, &T)>,
) {
    for (view_entity, visible_entities) in &view_query {
        view_entities.clear();

        for (e, _) in visible_entities.iter::<With<U>>() {
            view_entities.insert(*e);
        }

        view_array_buffer.insert(
            view_entity,
            Lighting2dArrayBuffer::<T>::new(
                components
                    .iter()
                    .filter(|(entity, _)| view_entities.contains(entity))
                    .map(|(_, component)| component.clone())
                    .collect(),
                view_entities.len() as u32,
            ),
        );

        view_array_buffer
            .get_mut(&view_entity)
            .unwrap()
            .write(&render_device, &render_queue);
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
    point_light_array_buffer: Res<Lighing2dViewArrayBuffer<ExtractedPointLight2d>>,
    light_occluders_array_buffer: Res<Lighing2dViewArrayBuffer<ExtractedLightOccluder2d>>,
    view_query: Query<(Entity, &Lighting2dAuxiliaryTextures), With<ExtractedLighting2dSettings>>,
) {
    let (Some(view_uniforms), Some(lighting_settings)) = (
        view_uniforms.uniforms.binding(),
        lighting_settings.binding(),
    ) else {
        return;
    };

    let sampler = render_device.create_sampler(&SamplerDescriptor::default());

    for (entity, aux_textures) in &view_query {
        let (Some(point_lights), Some(light_occluders)) = (
            point_light_array_buffer.get(&entity),
            light_occluders_array_buffer.get(&entity),
        ) else {
            continue;
        };

        let (
            Some(point_lights),
            Some(point_lights_count),
            Some(light_occluders),
            Some(light_occluders_count),
        ) = (
            point_lights.data.binding(),
            point_lights.count.binding(),
            light_occluders.data.binding(),
            light_occluders.count.binding(),
        )
        else {
            continue;
        };

        commands.entity(entity).insert(Lighting2dSurfaceBindGroups {
            sdf: render_device.create_bind_group(
                "sdf_bind_group",
                &prepass_pipelines.sdf_layout,
                &BindGroupEntries::sequential((
                    view_uniforms.clone(),
                    light_occluders.clone(),
                    light_occluders_count.clone(),
                )),
            ),
            lighting: render_device.create_bind_group(
                "lighting2d_bind_group",
                &prepass_pipelines.lighting_layout,
                &BindGroupEntries::sequential((
                    view_uniforms.clone(),
                    lighting_settings.clone(),
                    point_lights.clone(),
                    point_lights_count.clone(),
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
