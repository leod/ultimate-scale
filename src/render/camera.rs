use nalgebra as na;

use std::collections::HashSet;

use glutin::{VirtualKeyCode, WindowEvent};

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
        }
    }
}

#[derive(Debug, Clone)]
pub struct Camera {
    pub viewport: na::Vector4<f32>,
    pub projection: na::Matrix4<f32>,
    pub view: na::Matrix4<f32>,
}

#[derive(Debug, Clone)]
pub struct EditCameraView {
    target: na::Point3<f32>,
    min_distance: f32,
    height: f32,
    yaw_radians: f32,
    pitch_radians: f32,
}

impl Camera {
    pub fn new(viewport: na::Vector2<f32>, projection: na::Matrix4<f32>) -> Camera {
        Camera {
            viewport: na::Vector4::new(0.0, 0.0, viewport.x, viewport.y),
            projection,
            view: na::Matrix4::identity(),
        }
    }

    pub fn unproject(&self, win: &na::Point3<f32>) -> na::Point3<f32> {
        // As in:
        // https://www.nalgebra.org/rustdoc_glm/src/nalgebra_glm/ext/matrix_projection.rs.html#163

        let transform = (self.projection * self.view)
            .try_inverse()
            .unwrap_or_else(na::Matrix4::zeros);

        let point = na::Vector4::new(
            2.0 * (win.x - self.viewport.x) / self.viewport.z - 1.0,
            2.0 * (self.viewport.w - win.y - self.viewport.y) / self.viewport.w - 1.0,
            2.0 * win.z - 1.0,
            1.0,
        );

        let result = transform * point;
        na::Point3::from(result.fixed_rows::<na::U3>(0) / result.w)
    }
}

impl EditCameraView {
    pub fn new() -> EditCameraView {
        EditCameraView {
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

pub struct Input {
    config: Config,
    pressed_keys: HashSet<VirtualKeyCode>,
    modifiers_state: glutin::ModifiersState,

    /// Height delta is changed when mouse wheel events are received, but
    /// applied only later in the update function.
    height_delta: f32,
}

impl Input {
    pub fn new(config: Config) -> Input {
        Input {
            config,
            pressed_keys: HashSet::new(),
            modifiers_state: Default::default(),
            height_delta: 0.0,
        }
    }

    fn cur_move_speed_per_sec(&self) -> f32 {
        self.config.move_units_per_sec
            * if self.pressed_keys.contains(&self.config.fast_move_key) {
                self.config.fast_move_multiplier
            } else {
                1.0
            }
    }

    pub fn update(&mut self, dt_secs: f32, camera: &mut EditCameraView) {
        let move_speed = dt_secs * self.cur_move_speed_per_sec();
        let mut translation = na::Vector3::zeros();

        if self.pressed_keys.contains(&self.config.forward_key) {
            translation += &na::Vector3::new(0.0, -move_speed, 0.0);
        }
        if self.pressed_keys.contains(&self.config.backward_key) {
            translation += &na::Vector3::new(0.0, move_speed, 0.0);
        }

        if self.pressed_keys.contains(&self.config.left_key) {
            translation += &na::Vector3::new(move_speed, 0.0, 0.0);
        }
        if self.pressed_keys.contains(&self.config.right_key) {
            translation += &na::Vector3::new(-move_speed, 0.0, 0.0);
        }

        if self.pressed_keys.contains(&self.config.zoom_in_key) {
            camera.height -= move_speed;
        }
        if self.pressed_keys.contains(&self.config.zoom_out_key) {
            camera.height += move_speed;
        }

        // Apply height change from mouse wheel events
        camera.height += self.height_delta;
        self.height_delta = 0.0;

        camera.height = camera.height.max(0.5).min(100.0);

        let rotation_z = na::Rotation3::from_axis_angle(
            &na::Vector3::z_axis(),
            camera.yaw_radians - std::f32::consts::PI / 2.0,
        );

        camera.target += rotation_z.transform_vector(&translation);

        let rotation_speed = dt_secs * self.config.rotate_degrees_per_sec.to_radians();

        if self.pressed_keys.contains(&self.config.rotate_cw_key) {
            camera.yaw_radians -= rotation_speed;
        }
        if self.pressed_keys.contains(&self.config.rotate_ccw_key) {
            camera.yaw_radians += rotation_speed;
        }
    }

    pub fn on_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { input, .. } => {
                if let Some(keycode) = input.virtual_keycode {
                    match input.state {
                        glutin::ElementState::Pressed => self.pressed_keys.insert(keycode),
                        glutin::ElementState::Released => self.pressed_keys.remove(&keycode),
                    };
                }

                self.modifiers_state = input.modifiers;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let dt_secs = 0.5;

                // TODO: Not sure what the different types of delta mean here
                let delta_float = match delta {
                    glutin::MouseScrollDelta::LineDelta(_x, y) => *y,
                    glutin::MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                };

                self.height_delta += dt_secs * self.cur_move_speed_per_sec() * delta_float;
            }
            _ => (),
        }
    }
}
