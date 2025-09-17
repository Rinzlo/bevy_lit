#import bevy_render::{
    maths::affine3_to_square,
    view::View,
}
#import bevy_lit::{
    view_transformations::{frag_to_world, world_to_uv},
    light2d_vertex_output::VertexOutput,
    light2d_view_bindings::{view, settings, voronoi_texture, voronoi_sampler},
    types::PointLight2d,
}

struct VertexInput {
    @builtin(vertex_index) index: u32,
    // NOTE: Instance-rate vertex buffer members prefixed with i_
    // NOTE: i_model_transpose_colN are the 3 columns of a 3x4 matrix that is the transpose of the
    // affine 4x3 model matrix.
    @location(0) i_model_transpose_col0: vec4<f32>,
    @location(1) i_model_transpose_col1: vec4<f32>,
    @location(2) i_model_transpose_col2: vec4<f32>,
    @location(3) i_color: vec4<f32>,
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let vertex_position = vec3<f32>(
        f32(in.index & 0x1u),
        f32((in.index & 0x2u) >> 1u),
        0.0
    );

    out.clip_position = view.clip_from_world * affine3_to_square(mat3x4<f32>(
        in.i_model_transpose_col0,
        in.i_model_transpose_col1,
        in.i_model_transpose_col2,
    )) * vec4<f32>(vertex_position, 1.0);
    out.uv = vertex_position.xy;
    out.color = in.i_color;

    return out;
}

@group(1) @binding(0) var<uniform> light: PointLight2d;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let pos = frag_to_world(in.clip_position / settings.scale, view).xy;

    let light_dist = distance(pos, light.center);

    if light_dist > light.radius {
        discard;
    }

    let sdf = get_distance(pos);

    var light_contrib = in.color.rgb * attenuation(light, light_dist);

    // inside occluder
    if sdf <= 0.0 {
        light_contrib *= select(0.0, 1.0, bool(settings.tint_occluders));
    } else {
        if bool(light.shadows_enabled) {
            light_contrib *= raymarch(pos, light.center);
        }
    }

    if settings.edge_intensity > 0.0 && sdf > 0.0 {
        let edge_intensity = 1.0 / sdf * settings.edge_intensity;
        light_contrib += light_contrib * edge_intensity * 1.0;
    }

    return vec4<f32>(light_contrib, sdf);
}

fn get_distance(pos: vec2<f32>) -> f32 {
    let uv = world_to_uv(vec3(pos, 0.0), view);
    let seed = textureSampleLevel(voronoi_texture, voronoi_sampler, uv, 0.0);
    let dist = length(pos - frag_to_world(seed / settings.scale, view).xy);
    // Determine if the pixel is inside or outside the shape
    return select(dist, -dist, seed.w == 1.0);
}

fn attenuation(light: PointLight2d, dist: f32) -> f32 {
    let s = dist / light.radius;
    if s > 1.0 {
        return 0.0;
    }
    let s2 = pow(s, 2.0);
    return pow(1.0 - s2, 2.0) / (1.0 + light.falloff * s2);
}

// Implementation follows the demo in this article
// https://www.rykap.com/2020/09/23/distance-fields
fn raymarch(ray_origin: vec2<f32>, ray_target: vec2<f32>) -> f32 {
    let config = settings.raymarch;
    let max_steps = config.max_steps;
    let sharpness = config.sharpness;
    let jitter = config.jitter;

    let ray_direction = normalize(ray_target - ray_origin);
    let stop_at = distance(ray_origin, ray_target);

    var ray_progress = 0.0;
    var light_contrib = 1.0;

    for (var i = 0u; i < max_steps; i++) {
        // ray found target
        if ray_progress > stop_at {
            return light_contrib;
        }

        let sdf = get_distance(ray_origin + ray_progress * ray_direction);

        // ray found occluder
        if sdf <= 0.0 {
            break;
        }

        light_contrib = min(light_contrib, sdf / ray_progress * sharpness);
        ray_progress += sdf * (1.0 - jitter) + jitter * fract(sdf * 43758.5453);
    }

    return 0.0;
}
