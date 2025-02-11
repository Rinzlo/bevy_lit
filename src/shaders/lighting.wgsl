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
    let penetration = 7.5;
    let pos = frag_to_world(in.position).xy;
    let signed_dist = get_distance(pos);
    let within_occluder = signed_dist <= 0.0;

    var lighting_color = vec4(settings.ambient_light.rgb, 1.0);

    if within_occluder && !bool(settings.tint_occluders) {
        return vec4(1.);
    }

    for (var i = 0u; i < lights_count; i++) {
        let light = lights[i];
        let light_dist = distance(pos, light.center);

        if light_dist < light.radius {
            var light_contrib = vec4(light.color.rgb, 1.0) * attenuation(light, light_dist);

            if bool(light.shadows_enabled) {
                if !within_occluder {
                    light_contrib *= raymarch(pos, light.center);
                }
            }

            if within_occluder {
                // - penetration should happen just on the side of the occluder that receives light
                // - it should penetrate `penetration` amount

                let normalized_dist = -signed_dist / penetration;
                let penetration_contrib = smoothstep(1.0, 0.0, normalized_dist);

                light_contrib *= penetration_contrib;
            }

            lighting_color += light_contrib;
        }
    }

    return lighting_color;
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
