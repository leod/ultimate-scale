/// Heavily inspired by:
/// https://github.com/glium/glium/blob/master/examples/deferred.rs
pub use crate::render::shadow::CreationError; // TODO

use log::info;

use nalgebra as na;

use glium::{implement_vertex, uniform, Surface};

use crate::render::{shadow, Context, Light, RenderLists, Resources};

#[derive(Debug, Clone, Default)]
pub struct Config;

const NUM_TEXTURES: usize = 3;

pub struct DeferredShading {
    config: Config,
    window_size: glutin::dpi::LogicalSize,

    scene_textures: [glium::texture::Texture2d; NUM_TEXTURES],
    depth_texture: glium::texture::DepthTexture2d,
    light_texture: glium::texture::Texture2d,

    scene_program: glium::Program,
    light_program: glium::Program,
    composition_program: glium::Program,

    quad_vertex_buffer: glium::VertexBuffer<QuadVertex>,
    quad_index_buffer: glium::IndexBuffer<u16>,

    shadow_mapping: Option<shadow::ShadowMapping>,
}

impl DeferredShading {
    pub fn create<F: glium::backend::Facade>(
        facade: &F,
        config: &Config,
        window_size: glutin::dpi::LogicalSize,
        shadow_mapping_config: &Option<shadow::Config>,
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

        info!("Creating deferred scene program");
        let scene_program = glium::Program::from_source(
            facade,
            // Vertex shader
            "
                #version 140

                uniform mat4 mat_model;
                uniform mat4 mat_view;
                uniform mat4 mat_projection;

                in vec3 position;
                in vec3 normal;

                smooth out vec4 frag_position;
                smooth out vec4 frag_normal;

                void main() {
                    frag_position = mat_model * vec4(position, 1.0);
                    frag_normal = vec4(mat3(mat_model) * normal, 1.0);
                    //frag_normal = vec4(transpose(inverse(mat3(mat_model))) * normal, 1.0);

                    gl_Position = mat_projection * mat_view * frag_position;
                }
            ",
            // Fragment shader
            "
                #version 140

                uniform vec4 color;

                smooth in vec4 frag_position;
                smooth in vec4 frag_normal;

                out vec4 f_output1;
                out vec4 f_output2;
                out vec4 f_output3;

                void main() {
                    f_output1 = frag_position;
                    f_output2 = frag_normal;
                    f_output3 = color;
                }
            ",
            None,
        )?;

        info!("Creating deferred light program");
        let light_program = glium::Program::from_source(
            facade,
            // Vertex shader
            "
                #version 140

                uniform mat4 mat_orthogonal;

                in vec4 position;
                in vec2 tex_coord;

                smooth out vec2 frag_tex_coord;

                void main() {
                    frag_tex_coord = tex_coord; 

                    gl_Position = mat_orthogonal * position;
                }
            ",
            // Fragment shader
            "
                #version 140

                uniform sampler2D position_texture;
                uniform sampler2D normal_texture;

                uniform vec3 light_position;
                uniform vec3 light_attenuation;
                uniform vec3 light_color;
                uniform float light_radius;

                smooth in vec2 frag_tex_coord;

                out vec4 f_color;

                void main() {
                    vec4 position = texture(position_texture, frag_tex_coord);
                    vec3 normal = normalize(texture(normal_texture, frag_tex_coord).xyz);

                    vec3 light_vector = light_position - position.xyz;
                    float light_distance = length(light_vector);

                    float diffuse = max(dot(normal, light_vector / light_distance), 0.0);

                    if (diffuse > 0.0) {
                        float attenuation = 1.0 / (
                            light_attenuation.x +
                            light_attenuation.y * light_distance +
                            light_attenuation.z * light_distance * light_distance
                        );
                        attenuation *= 1.0 - pow(light_distance / light_radius, 2.0);
                        attenuation = max(attenuation, 0.0);

                        diffuse *= attenuation;
                    }

                    float ambient = 0.3;
                    float radiance = diffuse;

                    f_color = vec4(light_color * radiance, 1.0);
                }
            ",
            None,
        )?;

        info!("Creating deferred composition program");
        let composition_program = glium::Program::from_source(
            facade,
            // Vertex shader
            "
                #version 140

                uniform mat4 mat_orthogonal;

                in vec4 position;
                in vec2 tex_coord;

                smooth out vec2 frag_tex_coord;

                void main() {
                    frag_tex_coord = tex_coord;

                    gl_Position = mat_orthogonal * position;
                }
            ",
            // Fragment shader
            "
                #version 140

                uniform sampler2D color_texture;
                uniform sampler2D lighting_texture;

                smooth in vec2 frag_tex_coord;

                out vec4 f_color;

                void main() {
                    vec3 color_value = texture(color_texture, frag_tex_coord).rgb;
                    vec3 lighting_value = texture(lighting_texture, frag_tex_coord).rgb;

                    f_color = vec4(color_value * lighting_value, 1.0);
                }
            ",
            None,
        )?;

        let quad_vertex_buffer = glium::VertexBuffer::new(
            facade,
            &[
                QuadVertex {
                    position: [0.0, 0.0, 0.0, 1.0],
                    tex_coord: [0.0, 0.0],
                },
                QuadVertex {
                    position: [1.0, 0.0, 0.0, 1.0],
                    tex_coord: [1.0, 0.0],
                },
                QuadVertex {
                    position: [1.0, 1.0, 0.0, 1.0],
                    tex_coord: [1.0, 1.0],
                },
                QuadVertex {
                    position: [0.0, 1.0, 0.0, 1.0],
                    tex_coord: [0.0, 1.0],
                },
            ],
        )?;

        let quad_index_buffer = glium::IndexBuffer::new(
            facade,
            glium::index::PrimitiveType::TrianglesList,
            &[0u16, 1, 2, 0, 2, 3],
        )?;

        let shadow_mapping = if let Some(config) = shadow_mapping_config {
            Some(shadow::ShadowMapping::create(facade, config, true)?)
        } else {
            None
        };

        info!("Deferred shading initialized");

        Ok(DeferredShading {
            config: config.clone(),
            window_size,
            scene_textures,
            depth_texture,
            light_texture,
            scene_program,
            light_program,
            composition_program,
            quad_vertex_buffer,
            quad_index_buffer,
            shadow_mapping,
        })
    }

    pub fn on_window_resize<F: glium::backend::Facade>(
        &mut self,
        facade: &F,
        new_window_size: glutin::dpi::LogicalSize,
    ) -> Result<(), CreationError> {
        info!(
            "Recreating textures for deferred shading with size {:?}",
            new_window_size
        );

        let rounded_size: (u32, u32) = new_window_size.into();

        self.window_size = new_window_size;
        self.scene_textures = [
            Self::create_texture(facade, rounded_size)?,
            Self::create_texture(facade, rounded_size)?,
            Self::create_texture(facade, rounded_size)?,
        ];
        self.depth_texture = glium::texture::DepthTexture2d::empty_with_format(
            facade,
            glium::texture::DepthFormat::F32,
            glium::texture::MipmapsOption::NoMipmap,
            rounded_size.0,
            rounded_size.1,
        )?;
        self.light_texture = Self::create_texture(facade, rounded_size)?;

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
        self.scene_pass(facade, resources, context, render_lists)?;
        self.light_pass(facade, &render_lists.lights)?;
        self.composition_pass(target)?;

        // Before rendering plain non-deferred objects, copy depth buffer to
        // main surface
        let framebuffer =
            glium::framebuffer::SimpleFrameBuffer::depth_only(facade, &self.depth_texture).unwrap(); // TODO: unwrap
        let rounded_size: (u32, u32) = self.window_size.into();
        target.blit_from_simple_framebuffer(
            &framebuffer,
            &glium::Rect {
                left: 0,
                bottom: 0,
                width: rounded_size.1,
                height: rounded_size.1,
            },
            &glium::BlitTarget {
                left: 0,
                bottom: 0,
                width: rounded_size.1 as i32,
                height: rounded_size.1 as i32,
            },
            glium::uniforms::MagnifySamplerFilter::Nearest,
        );
        //target.clear_depth(1.0);

        render_lists
            .plain
            .render(resources, context, &Default::default(), target)?;

        Ok(())
    }

    fn scene_pass<F: glium::backend::Facade>(
        &mut self,
        facade: &F,
        resources: &Resources,
        context: &Context,
        render_lists: &RenderLists,
    ) -> Result<(), glium::DrawError> {
        let output = &[
            ("f_output1", &self.scene_textures[0]),
            ("f_output2", &self.scene_textures[1]),
            ("f_output3", &self.scene_textures[2]),
        ];

        // TODO: How can we avoid having to create framebuffers in every frame?
        let mut framebuffer = glium::framebuffer::MultiOutputFrameBuffer::with_depth_buffer(
            facade,
            output.iter().cloned(),
            &self.depth_texture,
        )
        .unwrap(); // TODO: unwrap
        framebuffer.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);

        if let Some(shadow_mapping) = self.shadow_mapping.as_mut() {
            shadow_mapping.render_shadowed(
                facade,
                resources,
                context,
                render_lists,
                &mut framebuffer,
            )?;
        } else {
            render_lists.solid.render_with_program(
                resources,
                context,
                &Default::default(),
                &self.scene_program,
                &mut framebuffer,
            )?;
        }

        Ok(())
    }

    fn light_pass<F: glium::backend::Facade>(
        &mut self,
        facade: &F,
        lights: &[Light],
    ) -> Result<(), glium::DrawError> {
        let draw_params = glium::DrawParameters {
            blend: glium::Blend {
                color: glium::BlendingFunction::Addition {
                    source: glium::LinearBlendingFactor::One,
                    destination: glium::LinearBlendingFactor::One,
                },
                alpha: glium::BlendingFunction::Addition {
                    source: glium::LinearBlendingFactor::One,
                    destination: glium::LinearBlendingFactor::One,
                },
                constant_value: (1.0, 1.0, 1.0, 1.0),
            },
            ..Default::default()
        };

        let mut light_buffer = glium::framebuffer::SimpleFrameBuffer::new(
            facade,
            &self.light_texture,
            //&self.depth_texture,
        )
        .unwrap(); // TODO: unwrap
        light_buffer.clear_color(0.1, 0.1, 0.1, 1.0);

        for light in lights.iter() {
            let light_position: [f32; 3] = light.position.coords.into();
            let light_attenuation: [f32; 3] = light.attenuation.into();
            let light_color: [f32; 3] = light.color.into();

            let uniforms = uniform! {
                mat_orthogonal: self.orthogonal_projection(),
                position_texture: &self.scene_textures[0],
                normal_texture: &self.scene_textures[1],
                light_position: light_position,
                light_attenuation: light_attenuation,
                light_color: light_color,
                light_radius: light.radius,
            };

            light_buffer.draw(
                &self.quad_vertex_buffer,
                &self.quad_index_buffer,
                &self.light_program,
                &uniforms,
                &draw_params,
            )?;
        }

        Ok(())
    }

    fn composition_pass<S: glium::Surface>(
        &mut self,
        target: &mut S,
    ) -> Result<(), glium::DrawError> {
        let uniforms = uniform! {
            mat_orthogonal: self.orthogonal_projection(),
            color_texture: &self.scene_textures[2],
            lighting_texture: &self.light_texture,
        };

        target.draw(
            &self.quad_vertex_buffer,
            &self.quad_index_buffer,
            &self.composition_program,
            &uniforms,
            &Default::default(),
        )
    }

    fn orthogonal_projection(&self) -> [[f32; 4]; 4] {
        // Scale our unit size quad to screen size before orthogonal projection
        let mat_scaling = na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(
            self.window_size.width as f32,
            self.window_size.height as f32,
            1.0,
        ));
        let mat_orthogonal = na::Matrix4::new_orthographic(
            0.0,
            self.window_size.width as f32,
            0.0,
            self.window_size.height as f32,
            -1.0,
            1.0,
        ) * mat_scaling;
        let mat_orthogonal: [[f32; 4]; 4] = mat_orthogonal.into();

        mat_orthogonal
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
    tex_coord: [f32; 2],
}

implement_vertex!(QuadVertex, position, tex_coord);
