#[macro_use]
pub mod shader;
pub mod camera;
pub mod error;
pub mod instancing;
pub mod machine;
pub mod object;
pub mod pipeline;
pub mod render_list;
pub mod resources;
pub mod scene;
pub mod screen_quad;
pub mod fxaa;

pub use camera::{Camera, EditCameraView};
pub use error::{CreationError, DrawError};
pub use instancing::Instancing;
pub use object::Object;
pub use pipeline::Pipeline;
pub use render_list::{Instance, RenderList};
pub use resources::Resources;
pub use screen_quad::ScreenQuad;

use nalgebra as na;

#[derive(Debug, Clone)]
pub struct Context {
    pub camera: Camera,
    pub elapsed_time_secs: f32,
    pub tick_progress: f32,
    pub main_light_pos: na::Point3<f32>,
    pub main_light_center: na::Point3<f32>,
}

impl_uniform_input!(
    Context,
    self => {
        viewport: Vec4 => self.camera.viewport.into(),
        mat_projection: Mat4 => self.camera.projection.into(),
        mat_view: Mat4 => self.camera.view.into(),
        main_light_pos: Vec3 => self.main_light_pos.coords.into(),
        elapsed_time_secs: Float => self.elapsed_time_secs,
        tick_progress: Float => self.tick_progress,
    },
);
