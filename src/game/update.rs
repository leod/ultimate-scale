use std::time::Duration;

use glium::glutin;

use nalgebra as na;

use rendology::Camera;

use crate::config::{Config, ViewConfig};
use crate::edit::Editor;
use crate::edit_camera_view::{EditCameraView, EditCameraViewInput};
use crate::exec::TickTime;
use crate::input_state::InputState;
use crate::render;

pub struct Input {
    pub dt: Duration,
    pub window_events: Vec<(InputState, glutin::WindowEvent)>,
    pub input_state: InputState,
    pub target_size: (u32, u32),
}

pub struct Output {
    pub render_stage: render::Stage,
    pub render_context: render::Context,
}

pub struct Update {
    fov: f32,
    camera: Camera,
    edit_camera_view: EditCameraView,
    edit_camera_view_input: EditCameraViewInput,

    editor: Editor,
}

impl Update {
    pub fn new_editor(config: &Config, editor: Editor) -> Self {
        let fov = config.view.fov_degrees.to_radians() as f32;

        // TODO: Account for DPI in initialization
        let viewport_size = na::Vector2::new(
            config.view.window_size.width as f32,
            config.view.window_size.height as f32,
        );
        let camera = Camera::new(viewport_size, perspective_matrix(fov, &viewport_size));
        let edit_camera_view = EditCameraView::new();
        let edit_camera_view_input = EditCameraViewInput::new(&config.camera);

        Self {
            fov,
            camera,
            edit_camera_view,
            edit_camera_view_input,
            editor,
        }
    }

    fn update(&mut self, input: Input) -> Output {
        let viewport_size =
            na::Vector2::new(input.target_size.0 as f32, input.target_size.1 as f32);
        self.camera.viewport_size = viewport_size;
        self.camera.projection = perspective_matrix(self.fov, &viewport_size);

        for (input_state, window_event) in input.window_events.into_iter() {
            self.edit_camera_view_input.on_event(&window_event);
            self.editor.on_event(&input_state, &window_event);
        }

        self.editor.update(
            input.dt,
            &input.input_state,
            &self.camera,
            &mut self.edit_camera_view,
        );

        self.edit_camera_view_input.update(
            input.dt.as_secs_f32(),
            &input.input_state,
            &mut self.edit_camera_view,
        );
        self.camera.view = self.edit_camera_view.view();

        self.render()
    }

    fn render(&mut self) -> Output {
        let mut render_stage = render::Stage::default();
        self.editor.render(&mut render_stage);

        let main_light_pos = na::Point3::new(
            15.0 + 20.0 * (std::f32::consts::PI / 4.0).cos(),
            15.0 + 20.0 * (std::f32::consts::PI / 4.0).sin(),
            20.0,
        );

        render_stage.lights.push(rendology::Light {
            position: main_light_pos,
            attenuation: na::Vector3::new(1.0, 0.0, 0.0),
            color: na::Vector3::new(1.0, 1.0, 1.0),
            is_main: true,
            ..Default::default()
        });

        let render_context = render::Context {
            rendology: rendology::Context {
                camera: self.camera.clone(),
                main_light_pos,
                main_light_center: na::Point3::new(15.0, 15.0, 0.0),
                ambient_light: na::Vector3::new(0.3, 0.3, 0.3),
            },
            tick_time: TickTime::zero(),
        };

        Output {
            render_stage,
            render_context,
        }
    }
}

fn perspective_matrix(fov_radians: f32, viewport_size: &na::Vector2<f32>) -> na::Matrix4<f32> {
    let projection =
        na::Perspective3::new(viewport_size.x / viewport_size.y, fov_radians, 0.1, 10000.0);
    projection.to_homogeneous()
}
