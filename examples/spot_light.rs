use bevy::{
    color::palettes::tailwind::{GRAY_500, GRAY_700},
    prelude::*,
    window::PrimaryWindow,
};
use bevy_lit::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, Lighting2dPlugin))
        .insert_resource(ClearColor(Color::from(GRAY_500)))
        .add_systems(Startup, setup)
        .add_systems(Update, update_cursor_light)
        .run();
}

#[derive(Component)]
struct CursorLight;

const X_EXTENT: f32 = 700.;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((
        Camera2d,
        Lighting2dSettings {
            scale: 1.0,
            ..default()
        },
        AmbientLight2d {
            intensity: 0.2,
            ..default()
        },
    ));

    commands.spawn((
        CursorLight,
        SpotLight2d {
            intensity: 4.0,
            outer_radius: 1024.0,
            outer_angle: 15.0,
            ..default()
        },
        Transform::from_xyz(0.0, 512.0, 0.0)
            .with_rotation(Quat::from_rotation_z(-90_f32.to_radians())),
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
    let color = materials.add(Color::from(GRAY_700));
    let num_shapes = shapes.len();

    for (i, shape) in shapes.into_iter().enumerate() {
        commands.spawn((
            Mesh2d(shape),
            MeshMaterial2d(color.clone()),
            LightOccluder2d::default(),
            Transform::from_xyz(
                -X_EXTENT / 2. + i as f32 / (num_shapes - 1) as f32 * X_EXTENT,
                0.0,
                0.0,
            ),
        ));
    }
}

fn update_cursor_light(
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Lighting2dSettings>>,
    mut light_query: Query<&mut Transform, With<CursorLight>>,
) {
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    let Ok(window) = window_query.single() else {
        return;
    };

    let Ok(mut transform) = light_query.single_mut() else {
        return;
    };

    if let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor).ok())
        .map(|ray| ray.origin.truncate().extend(0.0))
    {
        look_at_2d(&mut transform, world_position.xy());
    }
}

pub fn look_at_2d(transform: &mut Transform, target: Vec2) {
    let delta = target - transform.translation.truncate();

    if delta.length_squared() > f32::EPSILON {
        let angle = delta.y.atan2(delta.x);
        transform.rotation = Quat::from_rotation_z(angle);
    }
}
