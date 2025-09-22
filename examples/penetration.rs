use bevy::{
    color::palettes::tailwind::{GRAY_200, GRAY_500},
    dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin, FrameTimeGraphConfig},
    prelude::*,
    window::PrimaryWindow,
};
use bevy_lit::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: bevy::window::PresentMode::Immediate,
                    ..default()
                }),
                ..default()
            }),
            Lighting2dPlugin,
            FpsOverlayPlugin {
                config: FpsOverlayConfig {
                    enabled: true,
                    frame_time_graph_config: FrameTimeGraphConfig {
                        enabled: false,
                        ..default()
                    },
                    ..default()
                },
            },
        ))
        .insert_resource(ClearColor(Color::from(GRAY_500)))
        .add_systems(Startup, setup)
        .add_systems(Update, update_cursor_light)
        .run();
}

#[derive(Component)]
struct CursorLight;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((
        Camera2d,
        Lighting2dSettings {
            penetration: PenetrationSettings {
                max: 20.0,
                intensity: 1.0,
                falloff: 1.0,
                sample_directions: 16,
                sample_steps: 8,
            },
            ..default()
        },
        AmbientLight2d {
            intensity: 0.2,
            ..default()
        },
    ));

    commands.spawn((
        SpotLight2d {
            intensity: 4.0,
            outer_radius: 512.0,
            radial_falloff: 1.0,
            color: Color::WHITE,
            ..default()
        },
        CursorLight,
    ));

    let rect = meshes.add(Rectangle::from_length(100.));

    commands.spawn((
        Mesh2d(rect.clone()),
        MeshMaterial2d(materials.add(Color::from(GRAY_200))),
        LightOccluder2d::default(),
        Transform::from_xyz(-100., 0., 0.),
    ));

    commands.spawn((
        Mesh2d(rect),
        MeshMaterial2d(materials.add(Color::from(GRAY_200))),
        LightOccluder2d::default(),
        Transform::from_xyz(100., 0., 0.),
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
