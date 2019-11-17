pub mod shader;

use glium::glutin;

pub use crate::render::pipeline::shadow::CreationError; // TODO

use crate::render::Resources;

// TODO: Move screen quad into `Resources` or something like that
use crate::render::pipeline::deferred;
use crate::render::pipeline::{Context, InstanceParams, RenderList};

#[derive(Debug, Clone, Default)]
pub struct Config {}

pub struct Glow {
    glow_texture: glium::texture::Texture2d,

    quad_vertex_buffer: glium::VertexBuffer<deferred::vertex::QuadVertex>,
    quad_index_buffer: glium::IndexBuffer<u16>,
}

impl Glow {
    pub fn create<F: glium::backend::Facade>(
        facade: &F,
        config: &Config,
        window_size: glutin::dpi::LogicalSize,
    ) -> Result<Self, CreationError> {
        let rounded_size: (u32, u32) = window_size.into();
        let glow_texture = Self::create_glow_texture(facade, rounded_size)?;

        let quad_vertex_buffer = glium::VertexBuffer::new(facade, deferred::vertex::QUAD_VERTICES)?;

        let quad_index_buffer = glium::IndexBuffer::new(
            facade,
            glium::index::PrimitiveType::TrianglesList,
            deferred::vertex::QUAD_INDICES,
        )?;

        Ok(Glow {
            glow_texture,
            quad_vertex_buffer,
            quad_index_buffer,
        })
    }

    pub fn on_window_resize<F: glium::backend::Facade>(
        &mut self,
        facade: &F,
        new_window_size: glutin::dpi::LogicalSize,
    ) -> Result<(), CreationError> {
        let rounded_size: (u32, u32) = new_window_size.into();
        self.glow_texture = Self::create_glow_texture(facade, rounded_size)?;

        Ok(())
    }

    fn create_glow_texture<F: glium::backend::Facade>(
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
