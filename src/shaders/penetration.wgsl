#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_lit::view_transformations::{frag_to_world, world_to_uv}

@group(0) @binding(1) var lighting_texture: texture_2d<f32>;
@group(0) @binding(2) var lighting_sampler: sampler;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let pos = frag_to_world(in.position).xy;
    let current = textureSample(lighting_texture, lighting_sampler, in.uv);
    let sdf = current.a;

    let penetration = 8.0;

    // outside occluder or smaller than penetration
    if sdf > 0.0 || sdf < -penetration {
        return vec4(current.rgb, 1.0);
    }

    return vec4(get_average_luminance(pos, penetration), 1.0);
}

fn get_average_luminance(pos: vec2<f32>, range: f32) -> vec3<f32> {
    let directions = array(
        vec2(1.0, 0.0), vec2(-1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, -1.0),
        vec2(1.0, 1.0), vec2(-1.0, 1.0), vec2(1.0, -1.0), vec2(-1.0, -1.0)
    );

    var avg = vec3<f32>(0.0);

    for (var i = 0; i < 8; i++) { // 8 key directions
        for (var d = 1; d <= i32(range); d += 1) { // Sample outward in steps
            let offset = directions[i] * f32(d);
            let uv = world_to_uv(vec3(pos + offset, 0.0));
            avg += textureSampleLevel(lighting_texture, lighting_sampler, uv, 0.0).rgb;
        }
    }

    return avg / (range * 8.0);
}
