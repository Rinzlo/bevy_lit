#import bevy_lit::{
    view_transformations::frag_to_world,
    light2d_common::{
        VertexOutput,
        settings,
        view,
        get_sdf,
        attenuation,
        raymarch
    },
}

struct SpotLight2d {
    inner_radius: f32,
    outer_radius: f32,
    radial_falloff: f32,
    inner_angle: f32,
    outer_angle: f32,
    angular_falloff: f32,
    shadows_enabled: u32,
}

@group(1) @binding(0) var<uniform> light: SpotLight2d;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let pos = frag_to_world(in.clip_position / settings.scale, view).xy;

    let light_dist = distance(pos, in.center);
    let radial_attenuation = attenuation(
        light.inner_radius,
        light.outer_radius,
        light.radial_falloff,
        light_dist
    );

    if radial_attenuation == 0.0 {
        discard;
    }

    let fragment_direction = normalize(in.center - pos);
    let dot_product = dot(in.direction, fragment_direction);
    let angle_diff = acos(clamp(dot_product, -1.0, 1.0));
    let angular_attenuation = attenuation(
        light.inner_angle,
        light.outer_angle,
        light.angular_falloff,
        angle_diff
    );

    if angular_attenuation == 0.0 {
        discard;
    }

    let sdf = get_sdf(pos);

    var light_contrib = in.color.rgb * radial_attenuation * angular_attenuation;

    // inside occluder
    if sdf <= 0.0 {
        light_contrib *= select(0.0, 1.0, bool(settings.tint_occluders));
    } else {
        if bool(light.shadows_enabled) {
            light_contrib *= raymarch(pos, in.center);
        }
    }

    if settings.edge_intensity > 0.0 && sdf > 0.0 {
        let edge_intensity = 1.0 / sdf * settings.edge_intensity;
        light_contrib += light_contrib * edge_intensity * 1.0;
    }

    return vec4<f32>(light_contrib, sdf);
}
