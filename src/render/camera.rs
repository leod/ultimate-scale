use nalgebra as na;

use std::collections::HashSet;

use glutin::{VirtualKeyCode, WindowEvent};

#[derive(Debug, Clone)]
pub struct Camera {
    projection: na::Matrix4<f32>,
    target: na::Point3<f32>,

    min_distance: f32,
    height: f32,
    yaw_radians: f32,
    pitch_radians: f32,
}

impl Camera {
    pub fn new(
        projection: na::Matrix4<f32>,
    ) -> Camera {
        Camera {
            projection,
            target: na::Point3::new(0.0, 0.0, 0.0),
            min_distance: 3.0,
            height: 3.0,
            yaw_radians: -std::f32::consts::PI / 2.0,
            pitch_radians: -std::f32::consts::PI / 8.0,
        }
    }

    pub fn projection(&self) -> na::Matrix4<f32> {
        self.projection
    }

    pub fn target(&self) -> na::Point3<f32> {
        self.target        
    }

    pub fn view(&self) -> na::Matrix4<f32> {
        let up = na::Vector3::new(0.0, 0.0, 1.0);

        na::Matrix4::look_at_rh(&self.eye(), &self.target, &up)
    }

    pub fn eye(&self) -> na::Point3<f32> {
        self.target + na::Vector3::new(
            self.min_distance * self.yaw_radians.cos(),
            self.min_distance * self.yaw_radians.sin(),
            self.height,
        )
    }
}

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
            move_units_per_sec: 2.0,
            fast_move_multiplier: 4.0,
            rotate_degrees_per_sec: 90.0,
        }
    }
}

pub struct Input {
    config: Config,
    pressed_keys: HashSet<VirtualKeyCode>,
    modifiers_state: glutin::ModifiersState,
}

impl Input {
    pub fn new(config: Config) -> Input {
        Input {
            config,
            pressed_keys: HashSet::new(),
            modifiers_state: Default::default(),
        }
    }

    pub fn update(&self, dt_secs: f32, camera: &mut Camera) {
        let move_speed = dt_secs * self.config.move_units_per_sec *
            if self.pressed_keys.contains(&self.config.fast_move_key) {
                self.config.fast_move_multiplier
            } else {
                1.0
            };

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
            camera.height += move_speed;
        }
        if self.pressed_keys.contains(&self.config.zoom_out_key) {
            camera.height -= move_speed;
        }

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
            WindowEvent::KeyboardInput { device_id: _, input } => {
                if let Some(keycode) = input.virtual_keycode {
                    match input.state {
                        glutin::ElementState::Pressed =>
                            self.pressed_keys.insert(keycode),
                        glutin::ElementState::Released =>
                            self.pressed_keys.remove(&keycode),
                    };
                }

                self.modifiers_state = input.modifiers;
            }
            _ => (),
        }
    }
}
