#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_lit::{
    types::{Lighting2dSettings, PointLight2d},
    view_transformations::{frag_to_world, world_to_uv, uv_to_world}
}

@group(0) @binding(1) var<uniform> settings: Lighting2dSettings;
@group(0) @binding(2) var<storage> lights: array<PointLight2d>;
@group(0) @binding(3) var<uniform> lights_count: u32;
@group(0) @binding(4) var flood_texture: texture_2d<f32>;
@group(0) @binding(5) var flood_sampler: sampler;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let pos = frag_to_world(in.position).xy;

    var lighting_color = vec4(settings.ambient_light.rgb, 1.0);

    if get_distance(pos) <= 0.0 {
        if !bool(settings.tint_occluders) {
            return vec4(1.);
        }

        return lighting_color;
    }

    for (var i = 0u; i < lights_count; i++) {
        let light = lights[i];

        let dist = distance(light.center, pos);

        if dist < light.radius {
            var raymarch_contrib = 1.0;

            if bool(light.shadows_enabled) {
                raymarch_contrib = raymarch(light, pos);
            }

            lighting_color += vec4(light.color.rgb, 1.0) *
                attenuation(light, dist) *
                raymarch_contrib;
        }
    }

    return lighting_color;
}

fn get_distance(pos: vec2<f32>) -> f32 {
    let uv = world_to_uv(vec3(pos, 0.0));
    let flood_uv = textureSampleLevel(flood_texture, flood_sampler, uv, 0.0).xy;
    var dist = distance(pos, uv_to_world(flood_uv).xy);
    // 0.7 is the treshold I've found to avoid light
    // leakage if the point light is inside the occluder
    if dist < 0.7 {
        dist = 0.0;
    }
    return dist;
}

fn square(x: f32) -> f32 {
    return x * x;
}

// Attribution: https://lisyarus.github.io/blog/posts/point-light-attenuation.html
fn attenuation(light: PointLight2d, dist: f32) -> f32 {
    let s = dist / light.radius;
    if s > 1.0 {
        return 0.0;
    }
    let s2 = square(s);
    return light.intensity * square(1 - s2) / (1 + light.falloff * s2);
}

// Implementation follows the demo of this article with some enhancements
// https://www.rykap.com/2020/09/23/distance-fields
fn raymarch(light: PointLight2d, ray_origin: vec2<f32>) -> f32 {
    let config = settings.raymarch;
    let max_steps = config.max_steps;
    let sharpness = config.sharpness;
    let jitter = config.jitter;

    let ray_direction = normalize(light.center - ray_origin);
    let stop_at = distance(ray_origin, light.center);

    var ray_progress = 0.0;
    var light_contrib = 1.0;

    for (var i = 0u; i < max_steps; i++) {
        // ray found target
        if ray_progress > stop_at {
            // 1.0 next to the light and 0.0 at light.radius away
            let fade_ratio = 1.0 - clamp(stop_at / light.radius, 0.0, 1.0);
            // fade off quadratically instead of linearly
            let distance_factor = pow(fade_ratio, 2.0);

            return light_contrib * distance_factor;
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
