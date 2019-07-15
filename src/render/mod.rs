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

#[derive(Default, Clone)]
pub struct RenderLists {
    pub solid: RenderList,
    pub transparent: RenderList,
}

impl RenderLists {
    pub fn new() -> RenderLists {
        Default::default()
    }

    pub fn clear(&mut self) {
        self.solid.clear();
        self.transparent.clear();
    }
}
