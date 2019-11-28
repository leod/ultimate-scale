//! Shadow mapping.
//!
//! Heavily inspired by:
//! https://github.com/glium/glium/blob/master/examples/shadow_mapping.rs

mod shaders;

use log::info;

use nalgebra as na;

use glium::{uniform, Surface};

use crate::render::pipeline::{Context, RenderPass, ScenePassComponent};
use crate::render::shader::{self, ToUniforms};
use crate::render::{self, scene, Camera, DrawError, Instancing, Resources};

pub use crate::render::CreationError;

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

pub struct ShadowMapping {
    config: Config,
    shadow_map_program: glium::Program,
    shadow_texture: glium::texture::DepthTexture2d,
}

impl RenderPass for ShadowMapping {
    fn clear_buffers<F: glium::backend::Facade>(&self, facade: &F) -> Result<(), DrawError> {
        let mut shadow_target =
            glium::framebuffer::SimpleFrameBuffer::depth_only(facade, &self.shadow_texture)?;

        shadow_target.clear_depth(1.0);

        Ok(())
    }
}

impl ScenePassComponent for ShadowMapping {
    /// Transforms a shader so that it shadows the scene.
    ///
    /// Note that the transformed shader will require additional uniforms,
    /// which are given by `ShadowMapping::scene_pass_uniforms`.
    fn core_transform<P, V>(
        &self,
        core: render::shader::Core<Context, P, V>,
    ) -> render::shader::Core<Context, P, V> {
        shaders::render_shadowed_core_transform(core)
    }
}

impl ShadowMapping {
    #[rustfmt::skip]
    pub fn create<F: glium::backend::Facade>(
        facade: &F,
        config: &Config,
    ) -> Result<ShadowMapping, CreationError> {
        // Shader for creating the shadow map from light source's perspective
        info!("Creating shadow map program");
        let shadow_map_program = shaders::depth_map_core_transform(
            scene::model::scene_core(),
        ).build_program(facade, shader::InstancingMode::Vertex)?;

        let shadow_texture = glium::texture::DepthTexture2d::empty(
            facade,
            config.shadow_map_size.x,
            config.shadow_map_size.y,
        )?;

        info!("Shadow mapping initialized");

        Ok(ShadowMapping {
            config: config.clone(),
            shadow_map_program,
            shadow_texture,
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

    /// Render scene from the light's point of view into depth buffer.
    pub fn shadow_pass<F: glium::backend::Facade>(
        &self,
        facade: &F,
        resources: &Resources,
        context: &Context,
        instancing: &Instancing<scene::model::Params>,
        instancing_glow: &Instancing<scene::model::Params>,
    ) -> Result<(), DrawError> {
        let mut shadow_target =
            glium::framebuffer::SimpleFrameBuffer::depth_only(facade, &self.shadow_texture)?;

        let light_projection = self.light_projection();
        let light_view = self.light_view(context);

        let camera = Camera {
            viewport: na::Vector4::new(0.0, 0.0, 0.0, 0.0), // dummy value
            projection: light_projection,
            view: light_view,
        };

        let light_context = Context { camera, ..*context };

        let params = glium::DrawParameters {
            backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
            depth: glium::Depth {
                test: glium::DepthTest::IfLessOrEqual,
                write: true,
                ..Default::default()
            },
            ..Default::default()
        };

        instancing.draw(
            resources,
            &self.shadow_map_program,
            &light_context.to_uniforms(),
            &params,
            &mut shadow_target,
        )?;
        instancing_glow.draw(
            resources,
            &self.shadow_map_program,
            &light_context.to_uniforms(),
            &params,
            &mut shadow_target,
        )?;

        Ok(())
    }

    /// Returns uniforms for binding the shadow map during scene pass.
    pub fn scene_pass_uniforms(&self, context: &Context) -> impl ToUniforms + '_ {
        let light_projection = self.light_projection();
        let light_view = self.light_view(context);
        let mat_light_view_projection: [[f32; 4]; 4] = (light_projection * light_view).into();
        let shadow_map = glium::uniforms::Sampler::new(&self.shadow_texture)
            .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest)
            .minify_filter(glium::uniforms::MinifySamplerFilter::Nearest);

        uniform! {
            mat_light_view_projection: mat_light_view_projection,
            shadow_map: shadow_map,
        }
    }
}
