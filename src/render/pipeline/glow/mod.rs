pub mod shader;

use log::info;

use glium::framebuffer::SimpleFrameBuffer;
use glium::uniforms::{MagnifySamplerFilter, MinifySamplerFilter, Sampler, SamplerWrapFunction};
use glium::{glutin, uniform, Surface};

use crate::render::pipeline::{
    CompositionPassComponent, Context, InstanceParams, RenderPass, ScenePassComponent,
};
use crate::render::{self, screen_quad, DrawError, ScreenQuad};

pub use crate::render::CreationError;

#[derive(Debug, Clone, Default)]
pub struct Config {}

pub struct Glow {
    glow_texture: glium::texture::Texture2d,
    glow_texture_back: glium::texture::Texture2d,
    blur_program: glium::Program,
    screen_quad: ScreenQuad,
}

impl RenderPass for Glow {
    fn clear_buffers<F: glium::backend::Facade>(&self, facade: &F) -> Result<(), DrawError> {
        let mut framebuffer =
            glium::framebuffer::SimpleFrameBuffer::new(facade, &self.glow_texture)?;
        framebuffer.clear_color(0.0, 0.0, 0.0, 0.0);

        Ok(())
    }
}

impl ScenePassComponent for Glow {
    fn core_transform<P: InstanceParams, V: glium::vertex::Vertex>(
        &self,
        core: render::shader::Core<(Context, P), V>,
    ) -> render::shader::Core<(Context, P), V> {
        shader::glow_map_core_transform(core)
    }

    fn output_textures(&self) -> Vec<(&'static str, &glium::texture::Texture2d)> {
        vec![("f_glow_color", &self.glow_texture)]
    }
}

impl CompositionPassComponent for Glow {
    fn core_transform(
        &self,
        core: render::shader::Core<(), screen_quad::Vertex>,
    ) -> render::shader::Core<(), screen_quad::Vertex> {
        shader::composition_core_transform(core)
    }
}

impl Glow {
    pub fn create<F: glium::backend::Facade>(
        facade: &F,
        config: &Config,
        window_size: glutin::dpi::LogicalSize,
    ) -> Result<Self, CreationError> {
        let rounded_size: (u32, u32) = window_size.into();
        let glow_texture = Self::create_texture(facade, rounded_size)?;
        let glow_texture_back = Self::create_texture(facade, rounded_size)?;

        info!("Creating blur program");
        let blur_program = shader::blur_core().build_program(facade)?;

        info!("Creating screen quad");
        let screen_quad = ScreenQuad::create(facade)?;

        Ok(Glow {
            glow_texture,
            glow_texture_back,
            blur_program,
            screen_quad,
        })
    }

    pub fn blur_pass<F: glium::backend::Facade>(&self, facade: &F) -> Result<(), DrawError> {
        let num_passes = 5;

        let glow_map = Sampler::new(&self.glow_texture)
            .magnify_filter(MagnifySamplerFilter::Linear)
            .minify_filter(MinifySamplerFilter::Linear)
            .wrap_function(SamplerWrapFunction::Clamp);
        let glow_map_back = Sampler::new(&self.glow_texture_back)
            .magnify_filter(MagnifySamplerFilter::Linear)
            .minify_filter(MinifySamplerFilter::Linear)
            .wrap_function(SamplerWrapFunction::Clamp);

        let mut glow_buffer = SimpleFrameBuffer::new(facade, &self.glow_texture)?;
        let mut glow_buffer_back = SimpleFrameBuffer::new(facade, &self.glow_texture_back)?;

        for _ in 0..num_passes {
            glow_buffer_back.draw(
                &self.screen_quad.vertex_buffer,
                &self.screen_quad.index_buffer,
                &self.blur_program,
                &uniform! {
                    horizontal: false,
                    glow_texture: glow_map,
                },
                &Default::default(),
            )?;

            glow_buffer.draw(
                &self.screen_quad.vertex_buffer,
                &self.screen_quad.index_buffer,
                &self.blur_program,
                &uniform! {
                    horizontal: true,
                    glow_texture: glow_map_back,
                },
                &Default::default(),
            )?;
        }

        Ok(())
    }

    pub fn on_window_resize<F: glium::backend::Facade>(
        &mut self,
        facade: &F,
        new_window_size: glutin::dpi::LogicalSize,
    ) -> Result<(), CreationError> {
        let rounded_size: (u32, u32) = new_window_size.into();
        self.glow_texture = Self::create_texture(facade, rounded_size)?;
        self.glow_texture_back = Self::create_texture(facade, rounded_size)?;

        Ok(())
    }

    pub fn composition_pass_uniforms(&self) -> impl glium::uniforms::Uniforms + '_ {
        uniform! {
            glow_texture: &self.glow_texture,
        }
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
