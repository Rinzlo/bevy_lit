#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_lit::{
    types::Lighting2dSettings,
    view_transformations::{frag_to_world, world_to_uv},
}

@group(0) @binding(1) var<uniform> settings: Lighting2dSettings;
@group(0) @binding(2) var lighting_texture: texture_2d<f32>;
@group(0) @binding(3) var lighting_sampler: sampler;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let pos = frag_to_world(in.position).xy;
    let current = textureSample(lighting_texture, lighting_sampler, in.uv);
    let sdf = current.a;
    let p = settings.penetration;

    // Skip if outside occluder or penetration range
    if sdf > 0.0 || sdf < -p.max {
        return vec4(current.rgb, 1.0);
    }

    var penetration_color = vec3(0.0);
    var total_weight = 0.0;

    // Sampling configuration
    let two_pi = 6.2831853;
    let angle_step = two_pi / f32(p.directions);

    for (var dir_index = 0u; dir_index < p.directions; dir_index++) {
        let angle = f32(dir_index) * angle_step;
        let direction = vec2(cos(angle), sin(angle));

        for (var i = 0u; i < p.steps; i++) {
            let t = (f32(i) + 0.5) / f32(p.steps); // [0.03125 ... 0.96875]
            let distance = t * p.max;
            let offset = direction * distance;
            let sample_pos = pos + offset;
            let uv = world_to_uv(vec3(sample_pos, 0.0));
            let sample = textureSample(lighting_texture, lighting_sampler, uv);

            // Smooth falloff weight
            let weight = pow(1.0 - t, p.falloff);
            penetration_color += sample.rgb * weight;
            total_weight += weight;
        }
    }

    // Normalize and apply intensity
    if total_weight > 0.0 {
        penetration_color = (penetration_color / total_weight) * p.intensity;
    }

    return vec4(penetration_color, 1.0);
}
