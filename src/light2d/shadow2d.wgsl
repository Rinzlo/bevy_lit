#import bevy_lit::{
    view_transformations::frag_to_world,
    light2d_common::{
        VertexOutput,
        settings,
        view,
        raymarch
    },
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let pos = frag_to_world(in.clip_position / settings.scale, view).xy;
    let shadow_contrib = raymarch(pos, in.translation_rotation.xy);
    return vec4<f32>(vec3(shadow_contrib), 1.0);
}

