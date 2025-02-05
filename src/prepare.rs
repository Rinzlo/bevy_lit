use bevy::{
    ecs::entity::{EntityHashMap, EntityHashSet},
    prelude::*,
    render::{
        render_resource::{GpuArrayBufferable, StorageBuffer, UniformBuffer},
        renderer::{RenderDevice, RenderQueue},
        view::RenderVisibleEntities,
    },
};

use crate::extract::ExtractedLighting2dSettings;

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
