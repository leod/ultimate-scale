pub mod camera;
pub mod machine;
pub mod object;
pub mod render_list;
pub mod resources;
pub mod shadow;

pub use camera::{Camera, EditCameraView};
pub use object::{Instance, InstanceParams, Object};
pub use render_list::RenderList;
pub use resources::Resources;

pub struct Context {
    pub camera: camera::Camera,
    pub elapsed_time_secs: f32,
}

#[derive(Default, Clone)]
pub struct RenderLists {
    pub solid: RenderList,
    pub solid_shadow: RenderList,
    pub transparent: RenderList,
}

impl RenderLists {
    pub fn new() -> RenderLists {
        Default::default()
    }

    pub fn clear(&mut self) {
        self.solid.clear();
        self.solid_shadow.clear();
        self.transparent.clear();
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
