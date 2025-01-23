#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_lit::{
    types::{LightOccluder2d},
    view_transformations::{frag_coord_to_ndc, position_ndc_to_world},
}

@group(0) @binding(1) var<storage> occluders: array<LightOccluder2d>;
@group(0) @binding(2) var<uniform> occluders_count: u32;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let pos = position_ndc_to_world(frag_coord_to_ndc(in.position)).xy;

    var sdf = occluder_sd(pos, occluders[0]);
    for (var i = 1u; i < occluders_count; i++) {
        let occluder = occluders[i];

        sdf = min(sdf, occluder_sd(pos, occluder));
    }

    return vec4(sdf, 0.0, 0.0, 1.0);
}

fn occluder_sd(p: vec2f, occluder: LightOccluder2d) -> f32 {
    let local_pos = occluder.center - p;
    let d = abs(local_pos) - occluder.half_size;

    return length(max(d, vec2f(0.))) + min(max(d.x, d.y), 0.);
}
