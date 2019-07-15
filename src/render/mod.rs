pub mod object;
pub mod resources;
pub mod camera;
pub mod render_list;
pub mod machine;

use nalgebra as na;
use glium::{self, uniform};

pub use object::{Object, Instance, InstanceParams};
pub use camera::Camera;
pub use resources::Resources;
pub use render_list::RenderList;

pub struct Context {
    pub camera: camera::Camera,
    pub elapsed_time_secs: f32,
}
