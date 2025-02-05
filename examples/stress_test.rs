use bevy::{
    color::palettes::tailwind::{GRAY_300, GRAY_800},
    prelude::*,
};
use bevy_lit::prelude::*;
use rand::{self, rngs::SmallRng, Rng, SeedableRng};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, Lighting2dPlugin))
        .insert_resource(ClearColor(Color::from(GRAY_300)))
        .add_systems(Startup, setup)
        .add_systems(Update, move_entities)
        .run();
}

#[derive(Component)]
struct Torch;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // lighting camera
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

    // spawn point light
    commands.spawn((
        Torch,
        PointLight2d {
            color: Color::srgb(1.0, 1.0, 1.0),
            intensity: 3.0,
            radius: 200.0,
            falloff: 2.0,
            ..default()
        },
    ));

    let mut rng = SmallRng::seed_from_u64(0);
    let mesh = Mesh2d(meshes.add(Circle::new(4.)));
    let material = MeshMaterial2d(materials.add(Color::from(GRAY_800)));

    // spawns 32732 light occluders
    for x in -128..128 {
        for y in -128..128 {
            if x == 0 || rng.random_bool(0.5) {
                continue;
            }

            commands.spawn((
                mesh.clone(),
                material.clone(),
                LightOccluder2d::default(),
                Transform::from_translation(Vec3::new((x * 16) as f32, (y * 16) as f32, -25.0)),
            ));
        }
    }
}

fn move_entities(
    mut torch_query: Query<&mut Transform, With<Torch>>,
    mut camera_query: Query<&mut Transform, (With<Camera>, Without<Torch>)>,
    time: Res<Time>,
) {
    let Ok(mut torch_transform) = torch_query.get_single_mut() else {
        return;
    };

    torch_transform.translation.y += 16.0 * time.delta_secs();

    let Ok(mut camera_transform) = camera_query.get_single_mut() else {
        return;
    };

    camera_transform.translation.y = torch_transform.translation.y;
}
