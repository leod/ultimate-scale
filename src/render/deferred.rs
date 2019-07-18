/// Heavily inspired by:
/// https://github.com/glium/glium/blob/master/examples/deferred.rs
pub use crate::render::shadow::CreationError; // TODO

use glium::{implement_vertex, uniform, Surface};

use crate::render::{Context, RenderLists, Resources};

#[derive(Debug, Clone, Default)]
pub struct Config;

const NUM_TEXTURES: usize = 3;

struct DeferredShading {
    config: Config,
    window_size: glutin::dpi::LogicalSize,

    scene_textures: [glium::texture::Texture2d; NUM_TEXTURES],
    depth_texture: glium::texture::DepthTexture2d,
    light_texture: glium::texture::Texture2d,

    scene_program: glium::Program,
    //light_program: glium::Program,
    //composition_program: glium::Program,

    quad_vertex_buffer: glium::VertexBuffer<QuadVertex>,
    quad_index_buffer: glium::IndexBuffer<u16>,
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
        ];

        let depth_texture = glium::texture::DepthTexture2d::empty_with_format(
            facade,
            glium::texture::DepthFormat::F32,
            glium::texture::MipmapsOption::NoMipmap,
            rounded_size.0,
            rounded_size.1,
        )?;

        let light_texture = Self::create_texture(facade, rounded_size)?;

        let scene_program = glium::Program::from_source(
            facade,
            // Vertex shader
            "
                #version 140

                uniform mat4 mat_model;
                uniform mat4 mat_view;
                uniform mat4 mat_projection;

                in vec4 position;
                in vec4 normal;

                smooth out vec4 frag_position;
                smooth out vec4 frag_normal;

                void main() {
                    frag_position = mat_model * position;
                    frag_normal = mat_model * normal;

                    gl_Position = mat_projection * mat_view * frag_position;
                }
            ",
            // Fragment shader
            "
                #version 140

                uniform vec4 color;

                smooth in vec4 frag_position;
                smooth in vec4 frag_normal;

                out vec4 output1;
                out vec4 output2;
                out vec4 output3;

                void main() {
                    output1 = vec4(frag_position);
                    output2 = vec4(frag_normal);
                    output3 = color;
                }
            ",
            None,
        )?;

        let quad_vertex_buffer = glium::VertexBuffer::new(
            facade,
            &[
                QuadVertex { position: [0.0, 0.0, 0.0, 1.0], tex_coord: [0.0, 0.0] },
                QuadVertex { position: [1.0, 0.0, 0.0, 1.0], tex_coord: [1.0, 0.0] },
                QuadVertex { position: [1.0, 1.0, 0.0, 1.0], tex_coord: [1.0, 1.0] },
                QuadVertex { position: [0.0, 1.0, 0.0, 1.0], tex_coord: [0.0, 1.0] },
            ]
        )?;

        let quad_index_buffer = glium::IndexBuffer::new(
            facade,
            glium::index::PrimitiveType::TrianglesList,
            &[0u16, 1, 2, 0, 2, 3]
        )?;

        Ok(DeferredShading {
            config: config.clone(),
            window_size,
            scene_textures,
            depth_texture,
            light_texture,
            scene_program,
            //light_program,
            //composition_program,
            quad_vertex_buffer,
            quad_index_buffer,
        })
    }

    pub fn render_frame<F: glium::backend::Facade, S: glium::Surface>(
        &mut self,
        facade: &F,
        resources: &Resources,
        context: &Context,
        render_lists: &RenderLists,
        target: &mut S,
    ) -> Result<(), glium::DrawError> {
        Ok(())
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

#[derive(Copy, Clone)]
struct QuadVertex {
    position: [f32; 4],
    tex_coord: [f32; 2]
}

implement_vertex!(QuadVertex, position, tex_coord);
