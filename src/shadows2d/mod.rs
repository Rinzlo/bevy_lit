use bevy::prelude::*;
use bevy_voronoi::prelude::{Voronoi2dPlugin, VoronoiMaterial, VoronoiView};

use crate::prelude::Lighting2dSettings;

pub struct Shadows2dPlugin;
impl Plugin for Shadows2dPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Voronoi2dPlugin).add_systems(
            PostUpdate,
            (
                update_voronoi_view,
                update_voronoi_material,
                remove_voronoi_material,
                remove_voronoi_view,
            ),
        );
    }
}

/// A light occluder component. Should be used alongside a Mesh2d
#[derive(Component, Clone, Debug, Default, Reflect)]
#[require(VoronoiMaterial)]
pub struct LightOccluder2d {
    /// Any texture with a transparent background. The occluder will take it's shape.
    pub occluder_mask: Handle<Image>,
}

impl LightOccluder2d {
    pub fn new(occluder_mask: Handle<Image>) -> Self {
        Self { occluder_mask }
    }
}

fn update_voronoi_view(
    mut query: Query<
        (&Lighting2dSettings, &mut VoronoiView),
        Or<(Added<Lighting2dSettings>, Changed<Lighting2dSettings>)>,
    >,
) {
    for (settings, mut voronoi_view) in &mut query {
        voronoi_view.scale = settings.scale;
    }
}

fn remove_voronoi_view(mut commands: Commands, mut removed: RemovedComponents<Lighting2dSettings>) {
    for entity in removed.read() {
        if let Ok(mut commands) = commands.get_entity(entity) {
            commands.remove::<VoronoiView>();
        }
    }
}

fn update_voronoi_material(
    mut query: Query<
        (&LightOccluder2d, &mut VoronoiMaterial),
        Or<(Added<LightOccluder2d>, Changed<LightOccluder2d>)>,
    >,
) {
    for (occluder, mut material) in &mut query {
        material.alpha_mask = occluder.occluder_mask.clone()
    }
}

fn remove_voronoi_material(
    mut commands: Commands,
    mut removed: RemovedComponents<LightOccluder2d>,
) {
    for entity in removed.read() {
        if let Ok(mut commands) = commands.get_entity(entity) {
            commands.remove::<VoronoiMaterial>();
        }
    }
}
