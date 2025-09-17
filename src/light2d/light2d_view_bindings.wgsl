#define_import_path bevy_lit::light2d_view_bindings

#import bevy_render::view::View
#import bevy_lit::types::Lighting2dSettings

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<uniform> settings: Lighting2dSettings;
@group(0) @binding(2) var voronoi_texture: texture_2d<f32>;
@group(0) @binding(3) var voronoi_sampler: sampler;


