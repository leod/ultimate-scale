/// Heavily inspired by:
/// https://github.com/glium/glium/blob/master/examples/shadow_mapping.rs

use nalgebra as na;

use crate::config::ViewConfig;

use crate::render::{RenderLists, Resources, Context};

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

struct ShadowMapping {
    config: Config,

    shadow_map_program: glium::Program,
    render_program: glium::Program,

    shadow_texture: glium::texture::DepthTexture2d,
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
                uniform mat4 mat_depth_bias_mvp;
                
                in vec4 position;
                in vec4 normal;

                out vec4 v_shadow_coord;
                out vec4 v_model_normal;

                void main() {
                    gl_Position = mat_projection
                        * mat_view
                        * mat_model
                        * vec4(position, 1.0);

                    v_shadow_coord = mat_depth_bias_mvp * position;
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
        })
    }

    pub fn render_frame<S: glium::Surface>(
        &self,
        resources: &Resources,
        context: &Context,
        render_lists: &RenderLists,
        target: &mut S,
    ) -> Result<(), glium::DrawError> {
        // Render scene from the light's point of view into depth buffer

        Ok(())
    }
}
