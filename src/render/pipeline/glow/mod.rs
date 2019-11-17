pub mod shader;

use log::info;

use glium::glutin;

use crate::render::pipeline::{Context, InstanceParams, RenderList};
use crate::render::{Resources, ScreenQuad};

pub use crate::render::CreationError;

#[derive(Debug, Clone, Default)]
pub struct Config {}

pub struct Glow {
    glow_texture: glium::texture::Texture2d,

    composition_program: glium::Program,

    screen_quad: ScreenQuad,
}

impl Glow {
    pub fn create<F: glium::backend::Facade>(
        facade: &F,
        config: &Config,
        window_size: glutin::dpi::LogicalSize,
    ) -> Result<Self, CreationError> {
        let rounded_size: (u32, u32) = window_size.into();
        let glow_texture = Self::create_texture(facade, rounded_size)?;

        info!("Creating glow composition program");
        let composition_core = shader::composition_core();
        let composition_program = composition_core.build_program(facade)?;

        info!("Creating screen quad");
        let screen_quad = ScreenQuad::create(facade)?;

        Ok(Glow {
            glow_texture,
            composition_program,
            screen_quad,
        })
    }

    pub fn glow_texture(&self) -> &glium::texture::Texture2d {
        &self.glow_texture
    }

    pub fn blur_glow_texture(&mut self) -> Result<(), glium::DrawError> {
        Ok(())
    }

    pub fn on_window_resize<F: glium::backend::Facade>(
        &mut self,
        facade: &F,
        new_window_size: glutin::dpi::LogicalSize,
    ) -> Result<(), CreationError> {
        let rounded_size: (u32, u32) = new_window_size.into();
        self.glow_texture = Self::create_texture(facade, rounded_size)?;

        Ok(())
    }

    fn create_texture<F: glium::backend::Facade>(
        facade: &F,
        size: (u32, u32),
    ) -> Result<glium::texture::Texture2d, CreationError> {
        Ok(glium::texture::Texture2d::empty_with_format(
            facade,
            glium::texture::UncompressedFloatFormat::F32F32F32,
            glium::texture::MipmapsOption::NoMipmap,
            size.0,
            size.1,
        )?)
    }
}
