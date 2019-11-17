pub mod deferred;
pub mod glow;
pub mod instance;
pub mod light;
pub mod render_list;
pub mod shadow;
pub mod simple;
pub mod wind;

use nalgebra as na;

use crate::config::ViewConfig;
use crate::render::{Camera, Resources};

use deferred::DeferredShading;
use glow::Glow;
use shadow::ShadowMapping;

pub use instance::{DefaultInstanceParams, Instance, InstanceParams};
pub use light::Light;
pub use render_list::RenderList;

#[derive(Debug, Clone)]
pub struct Context {
    pub camera: Camera,
    pub elapsed_time_secs: f32,
    pub tick_progress: f32,
    pub main_light_pos: na::Point3<f32>,
    pub main_light_center: na::Point3<f32>,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            camera: Default::default(),
            elapsed_time_secs: 0.0,
            tick_progress: 0.0,
            main_light_pos: na::Point3::origin(),
            main_light_center: na::Point3::origin(),
        }
    }
}

#[derive(Default, Clone)]
pub struct RenderLists {
    pub solid: RenderList<DefaultInstanceParams>,
    pub solid_wind: RenderList<wind::Params>,

    /// Transparent instances.
    pub transparent: RenderList<DefaultInstanceParams>,

    /// Non-shadowed instances.
    pub plain: RenderList<DefaultInstanceParams>,

    pub lights: Vec<Light>,

    /// Screen-space stuff.
    pub ortho: RenderList<DefaultInstanceParams>,
}

impl RenderLists {
    pub fn clear(&mut self) {
        self.solid.clear();
        self.solid_wind.clear();
        self.transparent.clear();
        self.plain.clear();
        self.lights.clear();
        self.ortho.clear();
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub shadow_mapping: Option<shadow::Config>,
    pub deferred_shading: Option<deferred::Config>,
    pub glow: Option<glow::Config>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            shadow_mapping: Some(Default::default()),
            deferred_shading: None, //Some(Default::default()),
            glow: Some(Default::default()),
        }
    }
}

pub struct Pipeline {
    shadow_mapping: Option<ShadowMapping>,
    deferred_shading: Option<DeferredShading>,
    glow: Option<Glow>,
}

impl Pipeline {
    pub fn create<F: glium::backend::Facade>(
        facade: &F,
        config: &Config,
        view_config: &ViewConfig,
    ) -> Result<Pipeline, CreationError> {
        let have_glow = config.glow.is_some();
        let shadow_mapping = config
            .shadow_mapping
            .as_ref()
            .map(|config| ShadowMapping::create(facade, config, false, have_glow))
            .transpose()
            .map_err(CreationError::ShadowMapping)?;

        let deferred_shading = config
            .deferred_shading
            .as_ref()
            .map(|deferred_shading_config| {
                DeferredShading::create(
                    facade,
                    &deferred_shading_config,
                    view_config.window_size,
                    &config.shadow_mapping,
                )
            })
            .transpose()
            .map_err(CreationError::DeferredShading)?;

        let glow = config
            .glow
            .as_ref()
            .map(|config| Glow::create(facade, config, view_config.window_size))
            .transpose()
            .map_err(CreationError::Glow)?;

        Ok(Pipeline {
            shadow_mapping,
            deferred_shading,
            glow,
        })
    }

    pub fn render<S: glium::Surface>(
        &mut self,
        display: &glium::backend::glutin::Display,
        resources: &Resources,
        context: &Context,
        render_lists: &mut RenderLists,
        target: &mut S,
    ) -> Result<(), glium::DrawError> {
        if let Some(deferred_shading) = &mut self.deferred_shading {
            profile!("deferred");

            let intensity = 1.0;
            render_lists.lights.push(Light {
                position: context.main_light_pos,
                attenuation: na::Vector3::new(1.0, 0.01, 0.00001),
                color: na::Vector3::new(intensity, intensity, intensity),
                radius: 160.0,
            });

            deferred_shading.render_frame(display, resources, context, render_lists, target)?;
        } else if let Some(shadow_mapping) = &mut self.shadow_mapping {
            profile!("shadow");

            shadow_mapping.render_frame(display, resources, context, render_lists, target)?;
        } else {
            profile!("straight");

            self.render_straight(resources, context, render_lists, target)?;
        }

        Ok(())
    }

    pub fn render_straight<S: glium::Surface>(
        &self,
        resources: &Resources,
        context: &Context,
        render_lists: &RenderLists,
        target: &mut S,
    ) -> Result<(), glium::DrawError> {
        let blend = glium::DrawParameters {
            blend: glium::draw_parameters::Blend::alpha_blending(),
            ..Default::default()
        };

        {
            profile!("solid");
            render_lists
                .solid
                .render(resources, context, &Default::default(), target)?;
        }

        {
            profile!("wind");
            render_lists.solid_wind.render_with_program(
                resources,
                context,
                &Default::default(),
                &resources.wind_program,
                target,
            )?;
        }

        {
            profile!("plain");
            render_lists
                .plain
                .render(resources, context, &Default::default(), target)?;
        }

        {
            profile!("transparent");
            render_lists
                .transparent
                .render(resources, context, &blend, target)?;
        }

        Ok(())
    }

    pub fn on_window_resize<F: glium::backend::Facade>(
        &mut self,
        facade: &F,
        new_window_size: glium::glutin::dpi::LogicalSize,
    ) -> Result<(), CreationError> {
        if let Some(deferred_shading) = self.deferred_shading.as_mut() {
            deferred_shading
                .on_window_resize(facade, new_window_size)
                .map_err(CreationError::DeferredShading)?;
        }

        if let Some(glow) = self.glow.as_mut() {
            glow.on_window_resize(facade, new_window_size)
                .map_err(CreationError::Glow)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum CreationError {
    ShadowMapping(shadow::CreationError),
    DeferredShading(deferred::CreationError),
    Glow(glow::CreationError),
}
