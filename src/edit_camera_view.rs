use nalgebra as na;

use glium::glutin::{self, VirtualKeyCode, WindowEvent};

use crate::input_state::InputState;

#[derive(Debug, Clone)]
pub struct Config {
    pub forward_key: VirtualKeyCode,
    pub left_key: VirtualKeyCode,
    pub backward_key: VirtualKeyCode,
    pub right_key: VirtualKeyCode,
    pub zoom_in_key: VirtualKeyCode,
    pub zoom_out_key: VirtualKeyCode,
    pub rotate_cw_key: VirtualKeyCode,
    pub rotate_ccw_key: VirtualKeyCode,
    pub fast_move_key: VirtualKeyCode,

    pub move_units_per_sec: f32,
    pub fast_move_multiplier: f32,

    pub rotate_degrees_per_sec: f32,
    pub fast_rotate_multiplier: f32,
    pub max_height: f32,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            forward_key: VirtualKeyCode::W,
            left_key: VirtualKeyCode::A,
            backward_key: VirtualKeyCode::S,
            right_key: VirtualKeyCode::D,
            zoom_in_key: VirtualKeyCode::PageUp,
            zoom_out_key: VirtualKeyCode::PageDown,
            rotate_cw_key: VirtualKeyCode::E,
            rotate_ccw_key: VirtualKeyCode::Q,
            fast_move_key: VirtualKeyCode::LShift,
            move_units_per_sec: 4.0,
            fast_move_multiplier: 4.0,
            rotate_degrees_per_sec: 90.0,
            fast_rotate_multiplier: 2.0,
            max_height: 500.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EditCameraView {
    target: na::Point3<f32>,
    min_distance: f32,
    height: f32,
    yaw_radians: f32,
    pitch_radians: f32,
}

impl EditCameraView {
    pub fn new() -> Self {
        Self {
            target: na::Point3::new(5.0, 5.0, 0.0),
            min_distance: 3.0,
            height: 10.0,
            yaw_radians: -std::f32::consts::PI / 2.0,
            pitch_radians: -std::f32::consts::PI / 8.0,
        }
    }

    pub fn target(&self) -> na::Point3<f32> {
        self.target
    }

    pub fn set_target(&mut self, target: na::Point3<f32>) {
        self.target = target;
    }

    pub fn view(&self) -> na::Matrix4<f32> {
        let up = na::Vector3::new(0.0, 0.0, 1.0);

        na::Matrix4::look_at_rh(&self.eye(), &self.target, &up)
    }

    pub fn eye(&self) -> na::Point3<f32> {
        self.target
            + na::Vector3::new(
                self.min_distance * self.yaw_radians.cos(),
                self.min_distance * self.yaw_radians.sin(),
                self.height,
            )
    }
}

pub struct EditCameraViewInput {
    config: Config,

    /// Height delta is changed when mouse wheel events are received, but
    /// applied only later in the update function.
    height_delta: f32,
}

impl EditCameraViewInput {
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
            height_delta: 0.0,
        }
    }

    fn move_speed_per_sec(&self, input_state: &InputState) -> f32 {
        self.config.move_units_per_sec
            * if input_state.is_key_pressed(self.config.fast_move_key) {
                self.config.fast_move_multiplier
            } else {
                1.0
            }
    }

    fn rotate_speed_per_sec(&self, input_state: &InputState) -> f32 {
        self.config.rotate_degrees_per_sec
            * if input_state.is_key_pressed(self.config.fast_move_key) {
                self.config.fast_rotate_multiplier
            } else {
                1.0
            }
    }

    pub fn update(&mut self, dt_secs: f32, input_state: &InputState, camera: &mut EditCameraView) {
        let move_speed = dt_secs * self.move_speed_per_sec(input_state);
        let mut translation = na::Vector3::zeros();

        if input_state.is_key_pressed(self.config.forward_key) {
            translation += &na::Vector3::new(0.0, -move_speed, 0.0);
        }
        if input_state.is_key_pressed(self.config.backward_key) {
            translation += &na::Vector3::new(0.0, move_speed, 0.0);
        }

        if input_state.is_key_pressed(self.config.left_key) {
            translation += &na::Vector3::new(move_speed, 0.0, 0.0);
        }
        if input_state.is_key_pressed(self.config.right_key) {
            translation += &na::Vector3::new(-move_speed, 0.0, 0.0);
        }

        if input_state.is_key_pressed(self.config.zoom_in_key) {
            camera.height -= move_speed;
        }
        if input_state.is_key_pressed(self.config.zoom_out_key) {
            camera.height += move_speed;
        }

        // Apply height change from mouse wheel events
        camera.height += 0.25 * self.move_speed_per_sec(input_state) * self.height_delta;
        self.height_delta = 0.0;

        camera.height = camera.height.max(0.5).min(self.config.max_height);

        let rotation_z = na::Rotation3::from_axis_angle(
            &na::Vector3::z_axis(),
            camera.yaw_radians - std::f32::consts::PI / 2.0,
        );

        camera.target += rotation_z.transform_vector(&translation);

        let rotate_speed = dt_secs * self.rotate_speed_per_sec(input_state).to_radians();

        if input_state.is_key_pressed(self.config.rotate_cw_key) {
            camera.yaw_radians -= rotate_speed;
        }
        if input_state.is_key_pressed(self.config.rotate_ccw_key) {
            camera.yaw_radians += rotate_speed;
        }
    }

    pub fn on_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::MouseWheel { delta, .. } => {
                // TODO: Not sure what the different types of delta mean here
                let delta_float = match delta {
                    glutin::MouseScrollDelta::LineDelta(_x, y) => *y,
                    glutin::MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                };

                self.height_delta += delta_float;
            }
            _ => (),
        }
    }
}
