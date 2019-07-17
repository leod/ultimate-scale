/// Heavily inspired by:
/// https://github.com/glium/glium/blob/master/examples/shadow_mapping.rs
use log::info;

use nalgebra as na;

use glium::{implement_vertex, uniform, Surface};

use crate::config::ViewConfig;

use crate::render::{Camera, Context, RenderLists, Resources};

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

pub struct ShadowMapping {
    config: Config,

    shadow_map_program: glium::Program,
    render_program: glium::Program,
    debug_shadow_map_program: glium::Program,
    shadow_texture: glium::texture::DepthTexture2d,

    light_pos: na::Point3<f32>,
    light_center: na::Point3<f32>,

    debug_vertex_buffer: glium::VertexBuffer<DebugVertex>,
    debug_index_buffer: glium::IndexBuffer<u16>,
}

impl ShadowMapping {
    pub fn create<F: glium::backend::Facade>(
        facade: &F,
        config: &Config,
    ) -> Result<ShadowMapping, CreationError> {
        // Shaders for creating the shadow map from light source's perspective
        info!("Creating shadow map program");
        let shadow_map_program = glium::Program::from_source(
            facade,
            // Vertex shader
            "
                #version 330 core

                uniform mat4 mat_model;
                uniform mat4 mat_view;
                uniform mat4 mat_projection;

                in vec3 position;

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
            None,
        )?;

        // Shaders for rendering the shadowed scene
        info!("Creating shadow render program");
        let render_program = glium::Program::from_source(
            facade,

            // Vertex shader
            "
                #version 330 core

                uniform mat4 mat_model;
                uniform mat4 mat_view;
                uniform mat4 mat_projection;
                uniform mat4 mat_light_mvp;
                uniform vec3 light_pos;
                
                in vec3 position;
                in vec3 normal;

                out vec4 v_world_pos;
                out vec3 v_world_normal;
                out vec4 v_shadow_coord;

                void main() {
                    v_world_pos = mat_model * vec4(position, 1.0);

                    gl_Position = mat_projection
                        * mat_view
                        * v_world_pos;
 
                    v_world_normal = transpose(inverse(mat3(mat_model))) * normal;
                    v_shadow_coord = mat_light_mvp * vec4(position + 0.02*normal, 1.0);
                    //v_shadow_coord = mat_light_mvp * vec4(position, 1.0);
                }
            ",

            // Fragment shader
            "
                #version 330 core

                uniform sampler2D shadow_map;
                uniform vec3 light_pos;
                uniform vec4 color;

                in vec4 v_world_pos;
                in vec3 v_world_normal;
                in vec4 v_shadow_coord;

                out vec4 f_color;

                float shadow_calculation(vec4 pos_light_space) {
                    vec3 light_dir = normalize(vec3(light_pos - v_world_pos.xyz));
                    //float bias = max(0.0055 * (1.0 - dot(v_world_normal, light_dir)), 0.005);
                    float bias = 0.0;

                    vec3 proj_coords = pos_light_space.xyz / pos_light_space.w;

                    proj_coords = proj_coords * 0.5 + 0.5;

                    if (proj_coords.z > 1.0)
                        return 1.0;

                    if (proj_coords.x < 0.0 || proj_coords.x > 1.0 || proj_coords.y < 0.0 || proj_coords.y > 1.0) {
                        return 1.0;
                    }

                    float closest_depth = texture(shadow_map, proj_coords.xy).r;
                    float current_depth = proj_coords.z;

                    return current_depth > closest_depth + bias ? 0.5 : 1.0;
                }

                void main() {
                    vec3 light_color = vec3(1, 1, 1);

                    float ambient = 0.3;

                    float diffuse = max(
                        dot(
                            normalize(v_world_normal),
                            normalize(light_pos - v_world_pos.xyz)
                        ),
                        0.05
                    );

                    float shadow = shadow_calculation(v_shadow_coord);

                    f_color = vec4((ambient + shadow * diffuse) * color.rgb, 1.0);
                }
            ",

            None
        )?;

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

        Ok(ShadowMapping {
            config: config.clone(),
            shadow_map_program,
            render_program,
            shadow_texture,
            debug_vertex_buffer,
            debug_index_buffer,
            debug_shadow_map_program,
            light_pos: na::Point3::new(10.0, 10.0, 20.0),
            light_center: na::Point3::new(15.0, 15.0, 0.0),
        })
    }

    fn light_projection(&self) -> na::Matrix4<f32> {
        let w = 20.0;
        na::Matrix4::new_orthographic(-w, w, -w, w, 10.0, 50.0)
    }

    fn light_view(&self) -> na::Matrix4<f32> {
        na::Matrix4::look_at_rh(
            &self.light_pos,
            &self.light_center,
            &na::Vector3::new(0.0, 0.0, 1.0),
        )
    }

    pub fn render_frame<F: glium::backend::Facade, S: glium::Surface>(
        &mut self,
        facade: &F,
        resources: &Resources,
        context: &Context,
        render_lists: &RenderLists,
        target: &mut S,
    ) -> Result<(), glium::DrawError> {
        // TODO: unwrap
        let mut shadow_target =
            glium::framebuffer::SimpleFrameBuffer::depth_only(facade, &self.shadow_texture)
                .unwrap();

        let t = context.elapsed_time_secs / 48.0;
        self.light_pos.x = 15.0 + 20.0 * t.cos();
        self.light_pos.y = 15.0 + 20.0 * t.sin();

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
            let mat_projection: [[f32; 4]; 4] = context.camera.projection.into();
            let mat_view: [[f32; 4]; 4] = context.camera.view.into();

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
                let light_mvp = light_projection * light_view * instance.params.transform;

                let mat_model: [[f32; 4]; 4] = instance.params.transform.into();
                let mat_light_mvp: [[f32; 4]; 4] = light_mvp.into();
                let color: [f32; 4] = instance.params.color.into();
                let light_pos: [f32; 3] = self.light_pos.coords.into();

                let shadow_map = glium::uniforms::Sampler::new(&self.shadow_texture)
                    .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest)
                    .minify_filter(glium::uniforms::MinifySamplerFilter::Nearest);

                let uniforms = uniform! {
                    mat_model: mat_model,
                    mat_view: mat_view,
                    mat_projection: mat_projection,
                    mat_light_mvp: mat_light_mvp,
                    color: color,
                    t: context.elapsed_time_secs,
                    light_pos: light_pos,
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

        // Render transparent objects
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
