#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var view_texture: texture_2d<f32>;
@group(0) @binding(1) var lighting_texture: texture_2d<f32>;
@group(0) @binding(2) var texture_sampler: sampler;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let light_frag = textureSample(lighting_texture, texture_sampler, in.uv);
    let scene_frag = textureSample(view_texture, texture_sampler, in.uv);

    let ambient_light = vec4(vec3(0.2), 1.0);

    return scene_frag * (light_frag + ambient_light);
}
