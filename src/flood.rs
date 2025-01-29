use bevy::{
    image::ImageSampler,
    prelude::*,
    render::{
        camera::RenderTarget,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        view::RenderLayers,
    },
};

const SDF_LAYER: usize = 11;

pub struct FloodPlugin;

impl Plugin for FloodPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<FloodMask>::default())
            .add_systems(
                Update,
                (spawn_flood_masks, spawn_flood_elements, resize_flood_mask),
            );
    }
}

#[derive(Component, Default)]
pub struct FloodCamera;

#[derive(Component, Clone)]
pub struct DerivedFloodCamera;

#[derive(Component, TypePath, ExtractComponent, Clone)]
pub struct FloodMask {
    pub handle: Handle<Image>,
}

#[derive(Component, Default)]
pub struct FloodElement;

fn spawn_flood_masks(
    mut commands: Commands,
    cameras: Query<(Entity, &Camera, &OrthographicProjection, &Transform), Added<FloodCamera>>,
    mut images: ResMut<Assets<Image>>,
) {
    for (entity, camera, projection, transform) in &cameras {
        let viewport_size = camera
            .logical_viewport_size()
            .expect("sdf camera should have a size")
            .as_uvec2();

        let size = Extent3d {
            width: viewport_size.x,
            height: viewport_size.y,
            ..default()
        };

        let texture_descriptor = TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba16Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let mut canvas = Image {
            texture_descriptor,
            sampler: ImageSampler::linear(),
            ..default()
        };

        canvas.resize(size);

        let handle = images.add(canvas);

        let camera = Camera {
            order: -1,
            target: RenderTarget::Image(handle.clone()),
            clear_color: ClearColorConfig::Custom(Color::NONE),
            ..camera.clone()
        };

        commands.entity(entity).insert(FloodMask { handle });
        commands.spawn((
            DerivedFloodCamera,
            Name::new("SDF Camera"),
            Camera2d,
            camera,
            projection.clone(),
            RenderLayers::layer(SDF_LAYER),
            *transform,
        ));
    }
}

fn resize_flood_mask(
    cameras: Query<(&Camera, &FloodMask), (With<FloodCamera>, Changed<Camera>)>,
    mut images: ResMut<Assets<Image>>,
) {
    for (camera, sdf_mask) in &cameras {
        let canvas = images
            .get_mut(&sdf_mask.handle)
            .expect("flood camera should handle shouldn't be empty");

        let viewport_size = camera
            .logical_viewport_size()
            .expect("flood camera viewport should have a size")
            .as_uvec2();

        if canvas.size() == viewport_size {
            continue;
        }

        canvas.resize(Extent3d {
            width: viewport_size.x,
            height: viewport_size.y,
            ..default()
        });
    }
}

fn spawn_flood_elements(
    mut commands: Commands,
    sdf_elements: Query<(Entity, Option<&RenderLayers>), Added<FloodElement>>,
) {
    for (entity, maybe_render_layer) in &sdf_elements {
        let main_layer = RenderLayers::layer(0);
        let render_layer = maybe_render_layer.unwrap_or(&main_layer);

        commands
            .entity(entity)
            .insert(render_layer.clone().with(SDF_LAYER));
    }
}
