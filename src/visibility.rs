use bevy::{
    prelude::*,
    render::{primitives::Aabb, view::NoFrustumCulling},
};

use crate::types::{LightOccluder2d, PointLight2d};

pub fn check_lighting_2d_artifacts_bounds(
    mut commands: Commands,
    light_occluders: Query<
        (Entity, &LightOccluder2d),
        (
            Or<(Without<Aabb>, Changed<LightOccluder2d>)>,
            Without<NoFrustumCulling>,
        ),
    >,
    point_lights: Query<
        (Entity, &PointLight2d),
        (
            Or<(Without<Aabb>, Changed<PointLight2d>)>,
            Without<NoFrustumCulling>,
        ),
    >,
) {
    for (entity, occluder) in &light_occluders {
        commands.entity(entity).try_insert(Aabb {
            center: Vec3::ZERO.into(),
            half_extents: occluder.half_size.extend(0.).into(),
        });
    }

    for (entity, point_light) in &point_lights {
        commands.entity(entity).try_insert(Aabb {
            center: Vec3::ZERO.into(),
            half_extents: Vec2::splat(point_light.radius).extend(0.).into(),
        });
    }
}
