/// Heavily inspired by:
/// https://github.com/glium/glium/blob/master/examples/deferred.rs

pub use crate::render::shadow::CreationError; // TODO

#[derive(Debug, Clone, Default)]
pub struct Config;

const NUM_TEXTURES: usize = 4;

struct DeferredShading {
    config: Config,
    window_size: glutin::dpi::LogicalSize,

    scene_textures: [glium::texture::Texture2d; NUM_TEXTURES],
    depth_texture: glium::texture::DepthTexture2d,
    light_texture: glium::texture::Texture2d,
}

impl DeferredShading {
    pub fn create<F: glium::backend::Facade>(
        facade: &F,
        config: &Config,
        window_size: glutin::dpi::LogicalSize,
    ) -> Result<DeferredShading, CreationError> {
        let rounded_size: (u32, u32) = window_size.into();

        let scene_textures = [
            Self::create_texture(facade, rounded_size)?,
            Self::create_texture(facade, rounded_size)?,
            Self::create_texture(facade, rounded_size)?,
            Self::create_texture(facade, rounded_size)?,
        ];

        let depth_texture = glium::texture::DepthTexture2d::empty_with_format(
            facade,
            glium::texture::DepthFormat::F32,
            glium::texture::MipmapsOption::NoMipmap,
            rounded_size.0,
            rounded_size.1,
        )?;

        let light_texture = Self::create_texture(facade, rounded_size)?;

        Ok(DeferredShading {
            config: config.clone(),
            window_size,
            scene_textures,
            depth_texture,
            light_texture,
        })
    }

    fn create_texture<F: glium::backend::Facade>(
        facade: &F,
        size: (u32, u32),
    ) -> Result<glium::texture::Texture2d, CreationError> {
        Ok(glium::texture::Texture2d::empty_with_format(
            facade,
            glium::texture::UncompressedFloatFormat::F32F32F32F32,
            glium::texture::MipmapsOption::NoMipmap,
            size.0,
            size.1,
        )?)
    }
}
