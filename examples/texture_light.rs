use bevy::{camera::ScalingMode, color::palettes::tailwind::*, prelude::*, window::PrimaryWindow};
use bevy_lit::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            Lighting2dPlugin,
        ))
        .insert_resource(ClearColor(Color::from(GRAY_600)))
        .add_systems(Startup, setup)
        .add_systems(Update, update_cursor_light)
        .run();
}

#[derive(Component)]
struct CursorLight;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedHorizontal {
                viewport_width: 320.0,
            },
            ..OrthographicProjection::default_2d()
        }),
        Lighting2dSettings {
            raymarch: RaymarchSettings {
                max_steps: 32,
                jitter_contrib: 0.0,
                sharpness: 64.,
            },
            scale: 0.125,
            ..default()
        },
        AmbientLight2d {
            intensity: 0.2,
            color: Color::from(BLUE_300),
        },
    ));

    commands.spawn((
        CursorLight,
        TextureLight2d {
            image: asset_server.load("light_mask.png"),
            color: Color::from(YELLOW_400),
            intensity: 0.5,
            ..default()
        },
        Sprite::sized(Vec2::splat(8.0)),
    ));

    let tile = meshes.add(Rectangle::from_length(16.));
    let material = materials.add(Color::from(GRAY_800));

    commands.spawn((
        Mesh2d(tile.clone()),
        MeshMaterial2d(material.clone()),
        LightOccluder2d::default(),
        Transform::from_xyz(-16.0, 0.0, 0.0),
    ));
    commands.spawn((
        Mesh2d(tile),
        MeshMaterial2d(material),
        LightOccluder2d::default(),
        Transform::from_xyz(16.0, 0.0, 0.0),
    ));
}

fn update_cursor_light(
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Lighting2dSettings>>,
    mut point_light_query: Query<&mut Transform, With<CursorLight>>,
) {
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    let Ok(window) = window_query.single() else {
        return;
    };

    let Ok(mut point_light_transform) = point_light_query.single_mut() else {
        return;
    };

    if let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor).ok())
        .map(|ray| ray.origin.truncate().extend(0.0))
    {
        point_light_transform.translation = world_position;
    }
}
