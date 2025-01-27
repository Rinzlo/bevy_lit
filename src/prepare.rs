use bevy::{
    ecs::entity::{EntityHashMap, EntityHashSet},
    prelude::*,
    render::{
        render_resource::{
            GpuArrayBufferable, StorageBuffer, TextureDescriptor, TextureDimension, TextureFormat,
            TextureUsages, UniformBuffer,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::{CachedTexture, TextureCache},
        view::{RenderVisibleEntities, ViewTarget},
    },
};

use crate::extract::ExtractedLighting2dSettings;

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

#[derive(Clone, Component)]
pub struct Lighting2dTextures {
    flip: bool,
    texture_a: CachedTexture,
    texture_b: CachedTexture,
}

impl Lighting2dTextures {
    pub fn input(&self) -> &CachedTexture {
        if self.flip {
            &self.texture_b
        } else {
            &self.texture_a
        }
    }

    pub fn output(&self) -> &CachedTexture {
        if self.flip {
            &self.texture_a
        } else {
            &self.texture_b
        }
    }

    pub fn flip(&mut self) {
        self.flip = !self.flip;
    }
}

pub fn prepare_lighting_auxiliary_textures(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    view_targets: Query<(Entity, &ViewTarget), With<ExtractedLighting2dSettings>>,
) {
    for (entity, view_target) in &view_targets {
        commands.entity(entity).insert(Lighting2dTextures {
            flip: false,
            texture_a: create_aux_texture(
                view_target,
                &mut texture_cache,
                &render_device,
                "lighting2d_texture_a",
            ),
            texture_b: create_aux_texture(
                view_target,
                &mut texture_cache,
                &render_device,
                "lighting2d_texture_b",
            ),
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
