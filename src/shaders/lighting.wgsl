#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_lit::{
    types::{Lighting2dSettings, PointLight2d},
    view_transformations::{frag_to_world, world_to_uv}
}

@group(0) @binding(1) var<uniform> settings: Lighting2dSettings;
@group(0) @binding(2) var<storage> lights: array<PointLight2d>;
@group(0) @binding(3) var<uniform> lights_count: u32;
@group(0) @binding(4) var flood_texture: texture_2d<f32>;
@group(0) @binding(5) var flood_sampler: sampler;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let pos = frag_to_world(in.position).xy;
    let sdf = get_distance(pos);

    var lighting_color = vec3(0.0);

    for (var i = 0u; i < lights_count; i++) {
        let light = lights[i];
        let light_dist = distance(pos, light.center);

        if light_dist > light.radius {
            continue;
        }

        var light_contrib = light.color.rgb * attenuation(light, light_dist);

        // inside occluder
        if sdf <= 0.0 {
            light_contrib *= select(1.0, 0.0, bool(settings.tint_occluders));
        } else {
            if bool(light.shadows_enabled) {
                light_contrib *= raymarch(pos, light.center);
            }
        }

        lighting_color += light_contrib;
    }

    if sdf > 0.0 {
        let edge_intensity = 1.0 / sdf * 0.0001;
        lighting_color += lighting_color * edge_intensity;
    }

    return vec4(lighting_color, sdf);
}

fn get_distance(pos: vec2<f32>) -> f32 {
    let uv = world_to_uv(vec3(pos, 0.0));
    let seed = textureSampleLevel(flood_texture, flood_sampler, uv, 0.0);
    var dist = length(pos - frag_to_world(seed).xy);
    // Determine if the pixel is inside or outside the shape
    let is_inside = seed.z == 1.0;
    // Signed distance: negative if inside, positive if outside
    return select(dist, -dist, is_inside);
}

// Attribution: https://lisyarus.github.io/blog/posts/point-light-attenuation.html
fn attenuation(light: PointLight2d, dist: f32) -> f32 {
    let s = dist / light.radius;
    if s > 1.0 {
        return 0.0;
    }
    let s2 = pow(s, 2.0);
    return light.intensity * pow(1.0 - s2, 2.0) / (1.0 + light.falloff * s2);
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

        let dist = get_distance(ray_origin + ray_progress * ray_direction);

        // ray found occluder
        if dist <= 0.0 {
            break;
        }

        light_contrib = min(light_contrib, dist / ray_progress * sharpness);
        ray_progress += dist * (1.0 - jitter) + jitter * fract(dist * 43758.5453);
    }

    return 0.0;
}
