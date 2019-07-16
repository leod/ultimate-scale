/// Heavily inspired by:
/// https://github.com/glium/glium/blob/master/examples/shadow_mapping.rs

use nalgebra as na;

use glium::{uniform, Surface};

use crate::config::ViewConfig;

use crate::render::{RenderLists, Camera, Resources, Context};

#[derive(Debug, Clone)]
pub struct Config {
    pub shadow_map_size: na::Vector2<u32>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            shadow_map_size: na::Vector2::new(1024, 1024),
        }
    }
}

#[derive(Debug)]
pub enum CreationError {
    TextureCreationError(glium::texture::TextureCreationError),
    ProgramCreationError(glium::program::ProgramCreationError),
    FrameBufferValidationError(glium::framebuffer::ValidationError),
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

struct ShadowMapping {
    config: Config,

    shadow_map_program: glium::Program,
    render_program: glium::Program,
    shadow_texture: glium::texture::DepthTexture2d,

    light_pos: na::Point3<f32>,
    light_center: na::Point3<f32>,
}

impl ShadowMapping {
    pub fn create<F: glium::backend::Facade>(
        facade: &F,
        config: &Config,
    ) -> Result<ShadowMapping, CreationError> {
        // Shaders for creating the shadow map from light source's perspective
        let shadow_map_program = glium::Program::from_source(
            facade,

            // Vertex shader
            "
                #version 330 core

                uniform mat4 mat_model;
                uniform mat4 mat_view;
                uniform mat4 mat_projection;

                in vec4 position;

                void main() {
                    gl_Position = mat_projection
                        * mat_view
                        * mat_model
                        * vec4(position, 1.0);
                }
            ",

            // Fragment shader
            "
                #version 330 core

                layout(location = 0) out float f_fragment_depth;

                void main() {
                    f_fragment_depth = gl_FragCoord.z;
                }
            ",

            None
        )?;

        // Shaders for rendering the shadowed scene
        let render_program = glium::Program::from_source(
            facade,

            // Vertex shader
            "
                #version 330

                uniform mat4 mat_model;
                uniform mat4 mat_view;
                uniform mat4 mat_projection;
                uniform mat4 mat_light_bias_mvp;
                
                in vec4 position;
                in vec4 normal;

                out vec4 v_shadow_coord;
                out vec4 v_model_normal;

                void main() {
                    gl_Position = mat_projection
                        * mat_view
                        * mat_model
                        * vec4(position, 1.0);

                    v_shadow_coord = mat_light_bias_mvp * position;
                    v_model_normal = normalize(mat_model * normal);
                }
            ",

            // Fragment shader
            "
                #version 

                uniform sampler2DShadow shadow_map;
                uniform vec4 light_pos;
                uniform vec4 color;

                in vec4 v_shadow_coord;
                in vec4 v_model_normal;

                out vec4 f_color;

                void main() {
                    vec3 light_color = vec3(1, 1, 1);

                    float luminosity = max(
                        dot(normalize(model_normal.xyz), normalize(light_pos)),
                        0.0
                    );

                    float visibility = texture(
                        shadow_map,
                        vec3(
                            shadow_coord.xy,
                            shadow_coord.z / shadow_coord.w
                        )
                    );

                    f_color = vec4(
                        max(luminosity * visibility, 0.05) * color * light_color,
                        1.0
                    );
                }
            ",

            None
        )?;

        let shadow_texture = glium::texture::DepthTexture2d::empty(
            facade,
            config.shadow_map_size.x,
            config.shadow_map_size.y,
        )?;

        Ok(ShadowMapping {
            config: config.clone(),
            shadow_map_program,
            render_program,
            shadow_texture,
            light_pos: na::Point3::new(10.0, 10.0, 5.0),
            light_center: na::Point3::new(0.0, 0.0, 0.0),
        })
    }

    fn light_projection(&self) -> na::Matrix4<f32> {
        let w = 4.0;
        na::Matrix4::new_orthographic(-w, w, -w, w, -10.0, 20.0)
    }

    fn light_view(&self) -> na::Matrix4<f32> {
        na::Matrix4::look_at_rh(
            &self.light_pos,
            &self.light_center,
            &na::Vector3::new(0.0, 0.0, 1.0),
        )
    }

    pub fn render_frame<F: glium::backend::Facade, S: glium::Surface>(
        &self,
        facade: &F,
        resources: &Resources,
        context: &Context,
        render_lists: &RenderLists,
        target: &mut S,
    ) -> Result<(), glium::DrawError> {
        // TODO: unwrap
        let mut shadow_target = glium::framebuffer::SimpleFrameBuffer::depth_only(
            facade,
            &self.shadow_texture,
        ).unwrap();

        let light_projection = self.light_projection();
        let light_view = self.light_view();

        // Render scene from the light's point of view into depth buffer
        {
            let w = 4.0;
            let camera = Camera {
                viewport: na::Vector4::new(0.0, 0.0, 0.0, 0.0), // dummy value
                projection: light_projection,
                view: light_view,
            };

            let light_context = Context {
                camera,
                .. *context
            };

            shadow_target.clear_color(1.0, 1.0, 1.0, 1.0);
            shadow_target.clear_depth(1.0);

            render_lists.solid.render_with_program(
                resources,
                &light_context,
                &Default::default(),
                &self.shadow_map_program,
                &mut shadow_target,   
            )?;
        }

        // Render scene from the camera's point of view
        {
            let bias_matrix = na::Matrix4::new(
                0.5, 0.0, 0.0, 0.0,
                0.0, 0.5, 0.0, 0.0,
                0.0, 0.0, 0.5, 0.0,
                0.5, 0.5, 0.5, 1.0,
            );

            let mat_projection: [[f32; 4]; 4] = context.camera.projection.into();
            let mat_view: [[f32; 4]; 4] = context.camera.view.into();

            let params = glium::DrawParameters {
                backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
                depth: glium::Depth {
                    test: glium::DepthTest::IfLessOrEqual,
                    write: true,
                    .. Default::default()
                },
                .. Default::default()
            };

            for instance in &render_lists.solid.instances {
                let light_bias_mvp = bias_matrix
                    * light_projection
                    * light_view
                    * instance.params.transform;

                let mat_model: [[f32; 4]; 4] = instance.params.transform.into();
                let mat_light_bias_mvp: [[f32; 4]; 4] = light_bias_mvp.into();
                let color: [f32; 4] = instance.params.color.into();

                let shadow_map = glium::uniforms::Sampler::new(&self.shadow_texture)
                    .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest)
                    .minify_filter(glium::uniforms::MinifySamplerFilter::Nearest)
                    .depth_texture_comparison(Some(glium::uniforms::DepthTextureComparison::LessOrEqual));

                let uniforms = uniform! {
                    mat_model: mat_model, 
                    mat_view: mat_view,
                    mat_projection: mat_projection,
                    mat_light_bias_mvp: mat_light_bias_mvp,
                    color: color,
                    t: context.elapsed_time_secs,
                    shadow_map: shadow_map,
                };

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

        // Render transparent objects
        render_lists.transparent.render(
            resources,
            context,
            &glium::DrawParameters {
                blend: glium::draw_parameters::Blend::alpha_blending(), 
                .. Default::default()
            },
            target,
        )?;

        Ok(())
    }
}
