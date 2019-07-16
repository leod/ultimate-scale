use crate::render::camera;
use crate::render::shadow;
use crate::edit::editor;

#[derive(Debug, Clone)]
pub struct ViewConfig {
    pub window_size: glutin::dpi::LogicalSize,
    pub fov_degrees: f64,
}

impl Default for ViewConfig {
    fn default() -> ViewConfig {
        ViewConfig {
            window_size: glutin::dpi::LogicalSize::new(1024.0, 768.0),
            fov_degrees: 90.0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RenderConfig {
    shadow_mapping: Option<shadow::Config>,
}

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub camera: camera::Config,
    pub view: ViewConfig,
    pub render: RenderConfig,
    pub editor: editor::Config,
}

