#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_lit::{
    types::PointLight2d,
    view_transformations::{frag_to_world, world_to_uv},
}

@group(0) @binding(1) var lighting_texture: texture_2d<f32>;
@group(0) @binding(2) var lighting_sampler: sampler;
@group(0) @binding(3) var<storage> lights: array<PointLight2d>;
@group(0) @binding(4) var<uniform> lights_count: u32;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let pos = frag_to_world(in.position).xy;
    let current = textureSample(lighting_texture, lighting_sampler, in.uv);
    let sdf = current.a;

    let penetration_max = 5.0;
    let penetration_intensity = 1.0;
    let penetration_falloff = 2.0;  // Higher values give sharper falloff

    // Early exit for areas outside the penetration range
    if sdf > 0.0 || sdf < -penetration_max {
        return vec4(current.rgb, 1.0);
    }

    // Calculate how deep we are in the penetration zone (0.0 to 1.0)
    let penetration_depth = abs(sdf) / penetration_max;
    // Apply non-linear falloff for more natural light penetration
    let strength_factor = pow(1.0 - penetration_depth, penetration_falloff) * penetration_intensity;

    var penetration_color = vec3(0.0);

    for (var i = 0u; i < lights_count; i++) {
        let light = lights[i];
        let light_dist = distance(pos, light.center);

        // Skip lights that can't reach this point
        if light_dist > light.radius + penetration_max {
            continue;
        }

        let light_dir = normalize(light.center - pos);

        // Original penetration detection logic
        var should_tint = false;
        for (var d = 0; d <= i32(penetration_max); d++) {
            let offset = light_dir * f32(d);
            let uv = world_to_uv(vec3(pos + offset, 0.0));
            let sample = textureSampleLevel(lighting_texture, lighting_sampler, uv, 0.0).rgb;

            if !is_black(sample) {
                should_tint = true;
                break;
            }
        }

        if should_tint {
            // Apply distance-based attenuation and color
            penetration_color += light.color.rgb * attenuation(light, light_dist);
        }
    }

    // Apply strength factor to control penetration intensity
    penetration_color *= strength_factor;

    // Optional: Maintain some of the original color/darkness for better visual blend
    let ambient_factor = 0.05 * (1.0 - penetration_depth);
    let final_color = penetration_color + current.rgb * ambient_factor;

    return vec4(final_color, 1.0);
}

fn is_black(color: vec3<f32>) -> bool {
    let epsilon = 0.001;
    return length(color) < epsilon;
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
