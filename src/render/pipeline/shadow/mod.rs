/// Heavily inspired by:
/// https://github.com/glium/glium/blob/master/examples/shadow_mapping.rs
mod shader;

use log::info;

use nalgebra as na;

use glium::{implement_vertex, uniform, Surface};

use crate::render::pipeline::instance::UniformsPair;
use crate::render::pipeline::{self, Context, InstanceParams, RenderLists};
use crate::render::{Camera, Resources};

#[derive(Debug, Clone)]
pub struct Config {
    pub shadow_map_size: na::Vector2<u32>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            shadow_map_size: na::Vector2::new(4096, 4096),
        }
    }
}

#[derive(Debug)]
pub enum CreationError {
    TextureCreationError(glium::texture::TextureCreationError),
    ProgramCreationError(glium::program::ProgramCreationError),
    FrameBufferValidationError(glium::framebuffer::ValidationError),
    VertexBufferCreationError(glium::vertex::BufferCreationError),
    IndexBufferCreationError(glium::index::BufferCreationError),
}

impl From<glium::texture::TextureCreationError> for CreationError {
    fn from(err: glium::texture::TextureCreationError) -> CreationError {
        CreationError::TextureCreationError(err)
    }
}

impl From<glium::program::ProgramCreationError> for CreationError {
    fn from(err: glium::program::ProgramCreationError) -> CreationError {
        CreationError::ProgramCreationError(err)
    }
}

impl From<glium::framebuffer::ValidationError> for CreationError {
    fn from(err: glium::framebuffer::ValidationError) -> CreationError {
        CreationError::FrameBufferValidationError(err)
    }
}

impl From<glium::vertex::BufferCreationError> for CreationError {
    fn from(err: glium::vertex::BufferCreationError) -> CreationError {
        CreationError::VertexBufferCreationError(err)
    }
}

impl From<glium::index::BufferCreationError> for CreationError {
    fn from(err: glium::index::BufferCreationError) -> CreationError {
        CreationError::IndexBufferCreationError(err)
    }
}

pub struct ShadowMapping {
    config: Config,

    shadow_map_program: glium::Program,
    render_program: glium::Program,
    debug_shadow_map_program: glium::Program,
    shadow_texture: glium::texture::DepthTexture2d,

    debug_vertex_buffer: glium::VertexBuffer<DebugVertex>,
    debug_index_buffer: glium::IndexBuffer<u16>,
}

impl ShadowMapping {
    #[rustfmt::skip]
    pub fn create<F: glium::backend::Facade>(
        facade: &F,
        config: &Config,
        deferred: bool,
    ) -> Result<ShadowMapping, CreationError> {
        assert!(!deferred, "TODO after shader refactorin");

        // Shaders for creating the shadow map from light source's perspective
        info!("Creating shadow map program");
        let shadow_map_program = shader::depth_map_core_transform(
            pipeline::simple::plain_core(),
        ).build_program(facade)?;

        // Shaders for rendering the shadowed scene
        info!("Creating shadow render program");
        let core = pipeline::simple::plain_core();
        let core = shader::render_core_transform(core);
        let core = pipeline::simple::diffuse_core_transform(core);
        println!("{}", core.link().vertex.compile());
        println!("{}", core.link().fragment.compile());

        let render_program = core.build_program(facade)?;

        let shadow_texture = glium::texture::DepthTexture2d::empty(
            facade,
            config.shadow_map_size.x,
            config.shadow_map_size.y,
        )?;

        let debug_vertex_buffer = glium::VertexBuffer::new(
            facade,
            &[
                DebugVertex::new([0.25, -1.0], [0.0, 0.0]),
                DebugVertex::new([0.25, -0.25], [0.0, 1.0]),
                DebugVertex::new([1.0, -0.25], [1.0, 1.0]),
                DebugVertex::new([1.0, -1.0], [1.0, 0.0]),
            ],
        )
        .unwrap(); // TODO: unwrap

        let debug_index_buffer = glium::IndexBuffer::new(
            facade,
            glium::index::PrimitiveType::TrianglesList,
            &[0u16, 1, 2, 0, 2, 3],
        )
        .unwrap(); // TODO: unwrap

        let debug_shadow_map_program = glium::Program::from_source(
            facade,
            // Vertex Shader
            "
                #version 140
                in vec2 position;
                in vec2 tex_coords;
                out vec2 v_tex_coords;
                void main() {
                    gl_Position = vec4(position, 0.0, 1.0);
                    v_tex_coords = tex_coords;
                }
            ",
            // Fragement Shader
            "
                #version 140
                uniform sampler2D tex;
                in vec2 v_tex_coords;
                out vec4 f_color;
                void main() {
                    f_color = vec4(texture(tex, v_tex_coords).rgb, 1.0);
                }
            ",
            None,
        )?;

        info!("Shadow mapping initialized");

        Ok(ShadowMapping {
            config: config.clone(),
            shadow_map_program,
            render_program,
            shadow_texture,
            debug_vertex_buffer,
            debug_index_buffer,
            debug_shadow_map_program,
        })
    }

    fn light_projection(&self) -> na::Matrix4<f32> {
        let w = 20.0;
        na::Matrix4::new_orthographic(-w, w, -w, w, 10.0, 50.0)
    }

    fn light_view(&self, context: &Context) -> na::Matrix4<f32> {
        na::Matrix4::look_at_rh(
            &context.main_light_pos,
            &context.main_light_center,
            &na::Vector3::new(0.0, 0.0, 1.0),
        )
    }

    pub fn render_shadowed<F: glium::backend::Facade, S: glium::Surface>(
        &mut self,
        facade: &F,
        resources: &Resources,
        context: &Context,
        render_lists: &RenderLists,
        target: &mut S,
    ) -> Result<(), glium::DrawError> {
        // TODO: unwrap
        // TODO: Can we do this in glium without recreating the
        //       `SimpleFrameBuffer` every frame?
        let mut shadow_target =
            glium::framebuffer::SimpleFrameBuffer::depth_only(facade, &self.shadow_texture)
                .unwrap();

        let light_projection = self.light_projection();
        let light_view = self.light_view(context);

        // Render scene from the light's point of view into depth buffer
        {
            let camera = Camera {
                viewport: na::Vector4::new(0.0, 0.0, 0.0, 0.0), // dummy value
                projection: light_projection,
                view: light_view,
            };

            let light_context = Context { camera, ..*context };

            shadow_target.clear_color(1.0, 1.0, 1.0, 1.0);
            shadow_target.clear_depth(1.0);

            render_lists.solid_shadow.render_with_program(
                resources,
                &light_context,
                &Default::default(),
                &self.shadow_map_program,
                &mut shadow_target,
            )?;
        }

        // Render scene from the camera's point of view
        {
            let params = glium::DrawParameters {
                backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
                depth: glium::Depth {
                    test: glium::DepthTest::IfLessOrEqual,
                    write: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            for instance in &render_lists.solid.instances {
                let mat_light_view_projection: [[f32; 4]; 4] =
                    (light_projection * light_view).into();
                let shadow_map = glium::uniforms::Sampler::new(&self.shadow_texture)
                    .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest)
                    .minify_filter(glium::uniforms::MinifySamplerFilter::Nearest);

                let uniforms = UniformsPair(
                    UniformsPair(context.uniforms(), instance.params.uniforms()),
                    uniform! {
                        mat_light_view_projection: mat_light_view_projection,
                        shadow_map: shadow_map,
                    },
                );

                let buffers = resources.get_object_buffers(instance.object);
                buffers.index_buffer.draw(
                    &buffers.vertex_buffer,
                    &self.render_program,
                    &uniforms,
                    &params,
                    target,
                )?;
            }
        }

        Ok(())
    }

    pub fn render_frame<F: glium::backend::Facade, S: glium::Surface>(
        &mut self,
        facade: &F,
        resources: &Resources,
        context: &Context,
        render_lists: &RenderLists,
        target: &mut S,
    ) -> Result<(), glium::DrawError> {
        self.render_shadowed(facade, resources, context, render_lists, target)?;

        // Render debug texture
        /*{
            let sampler = glium::uniforms::Sampler::new(&self.shadow_texture)
                .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest)
                .minify_filter(glium::uniforms::MinifySamplerFilter::Nearest);

            let uniforms = uniform! {
                tex: sampler,
            };

            target.clear_depth(1.0);
            target
                .draw(
                    &self.debug_vertex_buffer,
                    &self.debug_index_buffer,
                    &self.debug_shadow_map_program,
                    &uniforms,
                    &Default::default(),
                )
                .unwrap();
        }*/

        // Render plain objects
        render_lists
            .plain
            .render(resources, context, &Default::default(), target)?;

        // Render transparent objects
        // TODO: "Integration" with deferred shading
        render_lists.transparent.render(
            resources,
            context,
            &glium::DrawParameters {
                blend: glium::draw_parameters::Blend::alpha_blending(),
                ..Default::default()
            },
            target,
        )?;

        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
struct DebugVertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

impl DebugVertex {
    pub fn new(position: [f32; 2], tex_coords: [f32; 2]) -> DebugVertex {
        Self {
            position,
            tex_coords,
        }
    }
}

implement_vertex!(DebugVertex, position, tex_coords);
