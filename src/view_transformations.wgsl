#define_import_path bevy_lit::view_transformations

#import bevy_render::view::{
    View,
    frag_coord_to_ndc,
    position_ndc_to_world,
    position_world_to_ndc,
    ndc_to_uv,
    uv_to_ndc,
}

fn frag_to_world(frag_coord: vec4<f32>, view: View) -> vec3<f32> {
    return position_ndc_to_world(frag_coord_to_ndc(frag_coord, view.viewport), view.world_from_clip);
}

fn uv_to_world(uv: vec2<f32>, view: View) -> vec3<f32> {
    return position_ndc_to_world(vec3(uv_to_ndc(uv), 0.0), view.world_from_clip);
}

fn world_to_uv(world_pos: vec3<f32>, view: View) -> vec2<f32> {
    return ndc_to_uv(position_world_to_ndc(world_pos, view.clip_from_world).xy);
}
