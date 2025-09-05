use bevy::render::{
    render_resource::{
        Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    },
    renderer::RenderDevice,
    texture::{CachedTexture, TextureCache},
    view::ViewTarget,
};

pub fn create_aux_texture(
    view_target: &ViewTarget,
    texture_cache: &mut TextureCache,
    render_device: &RenderDevice,
    label: &'static str,
    down_sample: u32,
) -> CachedTexture {
    let size = view_target.main_texture().size();
    let size = Extent3d {
        width: size.width / down_sample,
        height: size.height / down_sample,
        depth_or_array_layers: size.depth_or_array_layers,
    };

    texture_cache.get(
        render_device,
        TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba16Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
    )
}
