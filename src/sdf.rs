use bevy::{
    image::ImageSampler,
    prelude::*,
    render::{
        camera::RenderTarget,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_resource::{
            AsBindGroup, Extent3d, TextureDescriptor, TextureDimension, TextureFormat,
            TextureUsages,
        },
        view::RenderLayers,
    },
};

const SDF_LAYER: usize = 11;

pub struct SdfPlugin;

impl Plugin for SdfPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<SdfMaskBindGroup>::default())
            .add_systems(
                Update,
                (spawn_sdf_masks, spawn_sdf_elements, resize_sdf_mask),
            );
    }
}

#[derive(Component, Default)]
pub struct SdfCamera;

#[derive(Component, Clone)]
pub struct DerivedSdfCamera;

#[derive(Component, TypePath, AsBindGroup, ExtractComponent, Clone)]
pub struct SdfMaskBindGroup {
    #[texture(0)]
    pub handle: Handle<Image>,
}

#[derive(Component, Default)]
pub struct SdfElement;

fn spawn_sdf_masks(
    mut commands: Commands,
    cameras: Query<(Entity, &Camera, &OrthographicProjection, &Transform), Added<SdfCamera>>,
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
            sampler: ImageSampler::nearest(),
            ..default()
        };

        canvas.resize(size);

        let handle = images.add(canvas);

        let camera = Camera {
            order: -1,
            target: RenderTarget::Image(handle.clone()),
            clear_color: ClearColorConfig::Custom(Color::srgba(-1., -1., -1., -1.)),
            ..camera.clone()
        };

        commands.entity(entity).insert(SdfMaskBindGroup { handle });
        commands.spawn((
            DerivedSdfCamera,
            Name::new("SDF Camera"),
            Camera2d,
            camera,
            projection.clone(),
            RenderLayers::layer(SDF_LAYER),
            *transform,
        ));
    }
}

fn resize_sdf_mask(
    cameras: Query<(&Camera, &SdfMaskBindGroup), (With<SdfCamera>, Changed<Camera>)>,
    mut images: ResMut<Assets<Image>>,
) {
    for (camera, sdf_mask) in &cameras {
        let canvas = images
            .get_mut(&sdf_mask.handle)
            .expect("sdf camera should handle shouldn't be empty");

        let viewport_size = camera
            .logical_viewport_size()
            .expect("sdf camera viewport should have a size")
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

fn spawn_sdf_elements(
    mut commands: Commands,
    sdf_elements: Query<(Entity, Option<&RenderLayers>), Added<SdfElement>>,
) {
    for (entity, maybe_render_layer) in &sdf_elements {
        let main_layer = RenderLayers::layer(0);
        let render_layer = maybe_render_layer.unwrap_or(&main_layer);

        commands
            .entity(entity)
            .insert(render_layer.clone().with(SDF_LAYER));
    }
}
