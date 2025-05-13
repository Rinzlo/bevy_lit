#define_import_path bevy_lit::types

struct RaymarchSettings {
    max_steps: u32,
    jitter: f32,
    sharpness: f32,
    _pad: u32
}

struct PenetrationSettings {
    max: f32,
    intensity: f32,
    falloff: f32,
    directions: u32,
    steps: u32,
}

struct Lighting2dSettings {
    raymarch: RaymarchSettings,
    penetration: PenetrationSettings,
    ambient_light: vec4<f32>,
    // blur circle of confusion diameter
    coc: f32,
    fixed_resolution: u32,
    tint_occluders: u32,
}

struct LightOccluder2d {
    center: vec2<f32>,
    half_size: vec2<f32>,
}

struct PointLight2d {
    center: vec2<f32>,
    color: vec4<f32>,
    falloff: f32,
    intensity: f32,
    radius: f32,
    shadows_enabled: u32,
}
