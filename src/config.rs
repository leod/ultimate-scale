use crate::edit;
use crate::exec;
use crate::render::camera;
use crate::render::pipeline::{deferred, shadow};

#[derive(Debug, Clone)]
pub struct ViewConfig {
    pub window_size: glutin::dpi::LogicalSize,
    pub fov_degrees: f64,
}

impl Default for ViewConfig {
    fn default() -> ViewConfig {
        ViewConfig {
            window_size: glutin::dpi::LogicalSize::new(640.0, 480.0),
            fov_degrees: 45.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RenderConfig {
    pub shadow_mapping: Option<shadow::Config>,
    pub deferred_shading: Option<deferred::Config>,
}

impl Default for RenderConfig {
    fn default() -> RenderConfig {
        RenderConfig {
            shadow_mapping: None, //Some(Default::default()),
            deferred_shading: Some(Default::default()),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub camera: camera::Config,
    pub view: ViewConfig,
    pub render: RenderConfig,
    pub editor: edit::Config,
    pub exec: exec::view::Config,
}
