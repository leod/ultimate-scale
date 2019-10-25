pub mod deferred;
pub mod instance;
pub mod light;
pub mod render_list;
pub mod shadow;
pub mod simple;
pub mod wind;

use nalgebra as na;

use crate::render::{Camera, Resources};

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
}

impl RenderLists {
    pub fn new() -> RenderLists {
        Default::default()
    }

    pub fn clear(&mut self) {
        self.solid.clear();
        self.solid_wind.clear();
        self.transparent.clear();
        self.plain.clear();
        self.lights.clear();
    }
}

// TODO: Factor out into some struct that also holds the necessary programs
pub fn render_frame_straight<S: glium::Surface>(
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
