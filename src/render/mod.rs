mod light2d_node;
mod post_process_node;
mod sdf_node;

use std::ops::Range;

use bevy::{
    core_pipeline::core_2d::{
        extract_core_2d_camera_phases,
        graph::{Core2d, Node2d},
    },
    math::FloatOrd,
    platform::collections::{HashMap, HashSet},
    prelude::*,
    render::{
        render_graph::{RenderGraphExt, RenderLabel, ViewNodeRunner},
        render_phase::{
            CachedRenderPipelinePhaseItem, DrawFunctionId, DrawFunctions, PhaseItem,
            PhaseItemExtraIndex, SortedPhaseItem, ViewSortedRenderPhases,
        },
        render_resource::{
            CachedRenderPipelineId, Extent3d, TextureDescriptor, TextureDimension, TextureFormat,
            TextureUsages,
        },
        renderer::RenderDevice,
        sync_world::MainEntity,
        texture::{CachedTexture, TextureCache},
        view::{ExtractedView, RetainedViewEntity, ViewTarget},
        Extract, Render, RenderApp, RenderSystems,
    },
};

use crate::{
    post_process::render::ExtractedLighting2dSettings,
    render::{
        light2d_node::Light2dDrawNode, post_process_node::Light2dPostProcessDrawNode,
        sdf_node::VoronoiDrawNode,
    },
    settings::Lighting2dSettings,
};

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub enum Light2d {
    SdfPass,
    LightPass,
    PostProcessPass,
}

pub struct Light2dRenderPlugin;
impl Plugin for Light2dRenderPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<VoronoiTextures>()
            .init_resource::<LightingTextures>()
            .init_resource::<ViewSortedRenderPhases<MaskPhase>>()
            .init_resource::<ViewSortedRenderPhases<Light2dPhase>>()
            .init_resource::<DrawFunctions<MaskPhase>>()
            .init_resource::<DrawFunctions<Light2dPhase>>()
            .add_systems(
                ExtractSchedule,
                extract_light2d_phases.after(extract_core_2d_camera_phases),
            )
            .add_systems(
                Render,
                prepare_lighting_textures.in_set(RenderSystems::PrepareBindGroups),
            )
            .add_render_graph_node::<ViewNodeRunner<VoronoiDrawNode>>(Core2d, Light2d::SdfPass)
            .add_render_graph_node::<ViewNodeRunner<Light2dDrawNode>>(Core2d, Light2d::LightPass)
            .add_render_graph_node::<ViewNodeRunner<Light2dPostProcessDrawNode>>(
                Core2d,
                Light2d::PostProcessPass,
            )
            .add_render_graph_edges(
                Core2d,
                (
                    Light2d::SdfPass,
                    Light2d::LightPass,
                    Node2d::EndMainPass,
                    Light2d::PostProcessPass,
                    Node2d::EndMainPassPostProcessing,
                ),
            );
    }
}

pub struct MaskPhase {
    pub sort_key: FloatOrd,
    pub pipeline: CachedRenderPipelineId,
    pub draw_function: DrawFunctionId,
    pub entity: (Entity, MainEntity),
    pub batch_range: Range<u32>,
    pub extra_index: PhaseItemExtraIndex,
    pub indexed: bool,
}

impl PhaseItem for MaskPhase {
    #[inline]
    fn entity(&self) -> Entity {
        self.entity.0
    }

    #[inline]
    fn main_entity(&self) -> MainEntity {
        self.entity.1
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn batch_range(&self) -> &Range<u32> {
        &self.batch_range
    }

    #[inline]
    fn batch_range_mut(&mut self) -> &mut Range<u32> {
        &mut self.batch_range
    }

    #[inline]
    fn extra_index(&self) -> PhaseItemExtraIndex {
        self.extra_index.clone()
    }

    #[inline]
    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl SortedPhaseItem for MaskPhase {
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        self.sort_key
    }

    fn indexed(&self) -> bool {
        self.indexed
    }
}

impl CachedRenderPipelinePhaseItem for MaskPhase {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

pub struct Light2dPhase {
    pub sort_key: FloatOrd,
    pub pipeline: CachedRenderPipelineId,
    pub draw_function: DrawFunctionId,
    pub entity: (Entity, MainEntity),
    pub batch_range: Range<u32>,
    pub extra_index: PhaseItemExtraIndex,
    pub indexed: bool,
}

impl PhaseItem for Light2dPhase {
    #[inline]
    fn entity(&self) -> Entity {
        self.entity.0
    }

    #[inline]
    fn main_entity(&self) -> MainEntity {
        self.entity.1
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn batch_range(&self) -> &Range<u32> {
        &self.batch_range
    }

    #[inline]
    fn batch_range_mut(&mut self) -> &mut Range<u32> {
        &mut self.batch_range
    }

    #[inline]
    fn extra_index(&self) -> PhaseItemExtraIndex {
        self.extra_index.clone()
    }

    #[inline]
    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl SortedPhaseItem for Light2dPhase {
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        self.sort_key
    }

    #[inline]
    fn indexed(&self) -> bool {
        self.indexed
    }
}

impl CachedRenderPipelinePhaseItem for Light2dPhase {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

pub fn extract_light2d_phases(
    cameras: Extract<Query<(Entity, &Camera), (With<Camera2d>, With<Lighting2dSettings>)>>,
    mut mask_phases: ResMut<ViewSortedRenderPhases<MaskPhase>>,
    mut light2d_phases: ResMut<ViewSortedRenderPhases<Light2dPhase>>,
    mut live_entities: Local<HashSet<RetainedViewEntity>>,
) {
    live_entities.clear();

    for (entity, camera) in &cameras {
        if !camera.is_active {
            continue;
        }

        let retained_view_entity = RetainedViewEntity::new(entity.into(), None, 0);

        mask_phases.insert_or_clear(retained_view_entity);
        light2d_phases.insert_or_clear(retained_view_entity);
        live_entities.insert(retained_view_entity);
    }

    // Clear out all dead views
    mask_phases.retain(|camera_entity, _| live_entities.contains(camera_entity));
    light2d_phases.retain(|camera_entity, _| live_entities.contains(camera_entity));
}

#[derive(Clone)]
pub struct FlipTexture {
    flip: bool,
    texture_a: CachedTexture,
    texture_b: CachedTexture,
}

#[derive(Resource, Deref, DerefMut, Default)]
pub struct LightingTextures(pub HashMap<RetainedViewEntity, FlipTexture>);

#[derive(Resource, Deref, DerefMut, Default)]
pub struct VoronoiTextures(pub HashMap<RetainedViewEntity, FlipTexture>);

impl FlipTexture {
    pub fn input(&self) -> &CachedTexture {
        if self.flip {
            &self.texture_b
        } else {
            &self.texture_a
        }
    }

    pub fn output(&self) -> &CachedTexture {
        if self.flip {
            &self.texture_a
        } else {
            &self.texture_b
        }
    }

    pub fn flip(&mut self) {
        self.flip = !self.flip;
    }
}

fn create_aux_texture(
    view_target: &ViewTarget,
    texture_cache: &mut TextureCache,
    render_device: &RenderDevice,
    label: &'static str,
    scale: f32,
) -> CachedTexture {
    let size = view_target.main_texture().size();
    let size = Extent3d {
        width: (size.width as f32 * scale) as u32,
        height: (size.height as f32 * scale) as u32,
        depth_or_array_layers: size.depth_or_array_layers,
    };

    texture_cache.get(
        render_device,
        TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba16Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
    )
}

pub fn prepare_lighting_textures(
    views: Query<(&ViewTarget, &ExtractedView, &ExtractedLighting2dSettings)>,
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    mut voronoi_textures: ResMut<VoronoiTextures>,
    mut lighting_textures: ResMut<LightingTextures>,
    mut live_entities: Local<HashSet<RetainedViewEntity>>,
) {
    live_entities.clear();

    for (view_target, extracted_view, settings) in &views {
        live_entities.insert(extracted_view.retained_view_entity);

        voronoi_textures.insert(
            extracted_view.retained_view_entity,
            FlipTexture {
                flip: false,
                texture_a: create_aux_texture(
                    view_target,
                    &mut texture_cache,
                    &render_device,
                    "voronoi_texture_a",
                    settings.scale,
                ),
                texture_b: create_aux_texture(
                    view_target,
                    &mut texture_cache,
                    &render_device,
                    "voronoi_texture_b",
                    settings.scale,
                ),
            },
        );

        lighting_textures.insert(
            extracted_view.retained_view_entity,
            FlipTexture {
                flip: false,
                texture_a: create_aux_texture(
                    view_target,
                    &mut texture_cache,
                    &render_device,
                    "lighting_texture_a",
                    settings.scale,
                ),
                texture_b: create_aux_texture(
                    view_target,
                    &mut texture_cache,
                    &render_device,
                    "lighting_texture_b",
                    settings.scale,
                ),
            },
        );
    }

    voronoi_textures.retain(|entity, _| live_entities.contains(entity));
    lighting_textures.retain(|entity, _| live_entities.contains(entity));
}
