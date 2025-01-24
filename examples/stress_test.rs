use bevy::{
    color::palettes::tailwind::{GRAY_300, GRAY_800},
    prelude::*,
};
use bevy_lit::prelude::*;
use rand::{self, rngs::SmallRng, Rng, SeedableRng};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, Lighting2dPlugin))
        .insert_resource(ClearColor(Color::srgb(0.0, 0.0, 0.0)))
        .add_systems(
            Startup,
            (spawn_camera, spawn_floor, spawn_barrels, spawn_light),
        )
        .add_systems(Update, move_entities)
        .run();
}

#[derive(Component)]
struct Torch;

fn spawn_light(mut commands: Commands) {
    commands
        .spawn((
            Torch,
            Sprite {
                custom_size: Some(Vec2::splat(8.)),
                color: Color::from(GRAY_800),
                ..default()
            },
        ))
        .insert(Transform::from_translation(Vec3::new(0.0, 0.0, -50.0)))
        .insert(PointLight2d {
            color: Color::srgb(1.0, 1.0, 1.0),
            intensity: 3.0,
            radius: 200.0,
            falloff: 2.0,
        });
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        OrthographicProjection {
            scale: 0.5,
            ..OrthographicProjection::default_2d()
        },
        Lighting2dSettings {
            blur: 32.0,
            raymarch: RaymarchSettings {
                max_steps: 32,
                jitter_contrib: 0.5,
                sharpness: 10.0,
            },
            ..default()
        },
    ));
}

fn spawn_floor(mut commands: Commands) {
    for x in -128..128 {
        for y in -128..128 {
            commands
                .spawn(Sprite {
                    custom_size: Some(Vec2::splat(16.)),
                    color: Color::from(GRAY_300),
                    ..default()
                })
                .insert(Transform::from_translation(Vec3::new(
                    (x * 16) as f32,
                    (y * 16) as f32,
                    -100.0,
                )));
        }
    }
}

fn spawn_barrels(mut commands: Commands) {
    let mut rng = SmallRng::seed_from_u64(0);

    // spawns 32732 light occluders
    for x in -128..128 {
        for y in -128..128 {
            if x == 0 || rng.gen_bool(0.5) {
                continue;
            }

            commands
                .spawn(Sprite {
                    custom_size: Some(Vec2::splat(8.)),
                    color: Color::from(GRAY_800),
                    ..default()
                })
                .insert(Transform::from_translation(Vec3::new(
                    (x * 16) as f32,
                    (y * 16) as f32,
                    -25.0,
                )))
                .insert(LightOccluder2d {
                    half_size: Vec2::splat(4.0),
                });
        }
    }
}

fn move_entities(
    mut torch_query: Query<&mut Transform, With<Torch>>,
    mut camera_query: Query<&mut Transform, (With<Camera>, Without<Torch>)>,
    time: Res<Time>,
) {
    if let Ok(mut camera_transform) = camera_query.get_single_mut() {
        if let Ok(mut torch_transform) = torch_query.get_single_mut() {
            camera_transform.translation.y =
                camera_transform.translation.y + 16.0 * time.delta_secs();

            torch_transform.translation.y = camera_transform.translation.y;
        }
    }
}
