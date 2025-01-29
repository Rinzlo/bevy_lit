#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var mask_texture: texture_2d<f32>;
@group(0) @binding(1) var mask_sampler: sampler;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let mask_sample = textureSample(mask_texture, mask_sampler, in.uv);

    if mask_sample.a == 0. {
        return vec4<f32>(-1.);
    }

    return vec4<f32>(in.uv, 0., 1.);
}
