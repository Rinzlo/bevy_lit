use bevy::color::palettes::tailwind::{BLUE_300, BLUE_600, GRAY_200, GRAY_900, YELLOW_600};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_lit::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, Lighting2dPlugin))
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

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((
        Camera2d,
        Lighting2dSettings {
            blur: 32.,
            raymarch: RaymarchSettings {
                max_steps: 64,
                jitter_contrib: 0.5,
                sharpness: 10.,
            },
            ..default()
        },
        AmbientLight2d {
            brightness: 0.1,
            color: Color::from(BLUE_300),
        },
    ));

    let shapes = [
        meshes.add(Circle::new(50.0)),
        meshes.add(Annulus::new(25.0, 50.0)),
        meshes.add(Capsule2d::new(25.0, 50.0)),
        meshes.add(Rhombus::new(75.0, 100.0)),
        meshes.add(Rectangle::new(50.0, 100.0)),
        meshes.add(RegularPolygon::new(50.0, 6)),
        meshes.add(Triangle2d::new(
            Vec2::Y * 50.0,
            Vec2::new(-50.0, -50.0),
            Vec2::new(50.0, -50.0),
        )),
    ];
    let num_shapes = shapes.len();

    for (i, shape) in shapes.into_iter().enumerate() {
        // Distribute colors evenly across the rainbow.
        let color = Color::from(GRAY_900);

        commands.spawn((
            Mesh2d(shape),
            MeshMaterial2d(materials.add(color)),
            LightOccluder2d,
            Transform::from_xyz(
                // Distribute shapes from -X_EXTENT/2 to +X_EXTENT/2.
                -X_EXTENT / 2. + i as f32 / (num_shapes - 1) as f32 * X_EXTENT,
                0.0,
                0.0,
            ),
        ));
    }

    commands
        .spawn((MovingLights, Transform::default(), Visibility::default()))
        .with_children(|builder| {
            let point_light = PointLight2d {
                intensity: 2.0,
                radius: 1100.0,
                falloff: 3.0,
                ..default()
            };

            builder.spawn((
                PointLight2d {
                    color: Color::from(BLUE_600),
                    ..point_light
                },
                Transform::from_xyz(-X_EXTENT + 50. / 2., 0.0, 0.0),
            ));

            builder.spawn((
                PointLight2d {
                    color: Color::from(BLUE_600),
                    ..point_light
                },
                Transform::from_xyz(X_EXTENT + 50. / 2., 0.0, 0.0),
            ));
        });

    commands.spawn((
        CursorLight,
        PointLight2d {
            intensity: 2.0,
            radius: 400.0,
            falloff: 10.0,
            color: Color::from(YELLOW_600),
            ..default()
        },
    ));
}

fn update_cursor_light(
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Lighting2dSettings>>,
    mut point_light_query: Query<&mut Transform, With<CursorLight>>,
) {
    let Ok((camera, camera_transform)) = camera_query.get_single() else {
        return;
    };

    let Ok(window) = window_query.get_single() else {
        return;
    };

    let Ok(mut point_light_transform) = point_light_query.get_single_mut() else {
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
