pub mod conduit;
pub mod deferred;
pub mod instance;
pub mod light;
pub mod render_list;
pub mod shadow;
pub mod simple;

use nalgebra as na;

use crate::render::{Camera, Resources};

pub use instance::{DefaultInstanceParams, Instance, InstanceParams};
pub use light::Light;
pub use render_list::RenderList;

#[derive(Debug, Clone)]
pub struct Context {
    pub camera: Camera,
    pub elapsed_time_secs: f32,
    pub main_light_pos: na::Point3<f32>,
    pub main_light_center: na::Point3<f32>,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            camera: Default::default(),
            elapsed_time_secs: 0.0,
            main_light_pos: na::Point3::origin(),
            main_light_center: na::Point3::origin(),
        }
    }
}

#[derive(Default, Clone)]
pub struct RenderLists {
    pub solid: RenderList<DefaultInstanceParams>,
    pub solid_conduit: RenderList<conduit::Params>,

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
        self.transparent.clear();
        self.plain.clear();
        self.lights.clear();
    }
}

pub fn render_frame_straight<S: glium::Surface>(
    resources: &Resources,
    context: &Context,
    render_lists: &RenderLists,
    target: &mut S,
) -> Result<(), glium::DrawError> {
    render_lists
        .solid
        .render(resources, context, &Default::default(), target)?;

    render_lists
        .plain
        .render(resources, context, &Default::default(), target)?;

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
