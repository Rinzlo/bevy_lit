use bevy::color::palettes::tailwind::{BLUE_300, BLUE_600, GRAY_200, YELLOW_600};
use bevy::dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin, FrameTimeGraphConfig};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
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
        .insert_resource(ClearColor(Color::from(GRAY_200)))
        .add_systems(Startup, setup)
        .add_systems(Update, update_cursor_light)
        .add_systems(FixedUpdate, update_moving_lights)
        .run();
}

#[derive(Component)]
struct CursorLight;

#[derive(Component)]
struct MovingLights;

const X_EXTENT: f32 = 700.;

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, assets: Res<AssetServer>) {
    commands.spawn((
        Camera2d,
        Lighting2dSettings {
            edge_intensity: 4.0,
            raymarch: RaymarchSettings {
                max_steps: 32,
                jitter_contrib: 0.5,
                sharpness: 10.0,
            },
            ..default()
        },
        AmbientLight2d {
            brightness: 0.1,
            color: Color::from(BLUE_300),
        },
    ));

    let lettering_handle = assets.load("abc.png");

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(411., 200.))),
        Sprite {
            image: lettering_handle.clone(),
            ..default()
        },
        LightOccluder2d::new(lettering_handle),
    ));

    commands
        .spawn((MovingLights, Transform::default(), Visibility::default()))
        .with_children(|builder| {
            let point_light = Light2d::Point {
                intensity: 2.0,
                outer_radius: 1100.0,
                inner_radius: 0.0,
                falloff: 3.0,
                color: Color::from(BLUE_600),
                shadows_enabled: true,
            };

            builder.spawn((
                point_light.clone(),
                Transform::from_xyz(-X_EXTENT + 50. / 2., 0.0, 0.0),
            ));

            builder.spawn((
                point_light,
                Transform::from_xyz(X_EXTENT + 50. / 2., 0.0, 0.0),
            ));
        });

    commands.spawn((
        CursorLight,
        Light2d::Point {
            intensity: 2.0,
            outer_radius: 400.0,
            inner_radius: 0.0,
            falloff: 10.0,
            color: Color::from(YELLOW_600),
            shadows_enabled: true,
        },
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

fn update_moving_lights(
    time: Res<Time>,
    mut point_light_query: Query<&mut Transform, With<MovingLights>>,
) {
    for mut transform in &mut point_light_query {
        transform.rotation *= Quat::from_rotation_z(time.delta_secs() / 12.0);
    }
}
