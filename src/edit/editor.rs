use std::collections::HashMap;

use nalgebra as na;

use glutin::{VirtualKeyCode, WindowEvent};

use crate::util::intersection::{ray_quad_intersection, Ray, Plane};
use crate::machine::grid;
use crate::machine::{Block, PlacedBlock, Machine};
use crate::render::{self, Camera, EditCameraView, RenderLists};

use crate::edit::Edit;

#[derive(Debug, Clone)]
pub struct Config {
    pub rotate_block_key: VirtualKeyCode,
    pub block_keys: HashMap<VirtualKeyCode, Block>,
    pub layer_keys: HashMap<VirtualKeyCode, isize>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            rotate_block_key: VirtualKeyCode::R,
            block_keys:
                vec![
                    (VirtualKeyCode::Key1, Block::PipeXY),
                    (VirtualKeyCode::Key2, Block::PipeSplitXY),
                    (VirtualKeyCode::Key3, Block::PipeBendXY),
                    (VirtualKeyCode::Key4, Block::Solid),
                ].into_iter().collect(),
            layer_keys:
                vec![
                    (VirtualKeyCode::F1, 0),
                    (VirtualKeyCode::F2, 1),
                    (VirtualKeyCode::F3, 2),
                    (VirtualKeyCode::F4, 3),
                ].into_iter().collect(),
        }
    }
}

pub struct Editor {
    config: Config,

    machine: Machine,

    place_block: PlacedBlock,

    current_layer: isize,
    mouse_window_pos: na::Point2<f32>,
    mouse_grid_pos: Option<grid::Point3>,

    left_mouse_button_pressed: bool,
    right_mouse_button_pressed: bool,
}

impl Editor {
    pub fn new(config: Config, size: grid::Vector3) -> Editor {
        Editor {
            config,
            machine: Machine::new(size),
            place_block: PlacedBlock {
                dir_xy: grid::Dir2(grid::Axis2::X, grid::Sign::Pos),
                block: Block::PipeXY,
            },
            current_layer: 0,
            mouse_window_pos: na::Point2::origin(),
            mouse_grid_pos: None,
            left_mouse_button_pressed: false,
            right_mouse_button_pressed: false,
        }
    }

    pub fn run_edit(&mut self, edit: Edit) {
        edit.run(&mut self.machine);
    }

    pub fn update(
        &mut self,
        dt_secs: f32,
        camera: &Camera,
        edit_camera_view: &mut EditCameraView,
    ) {
        self.update_mouse_grid_pos(camera, edit_camera_view);
        self.update_input();

        edit_camera_view.set_target(na::Point3::new(
            edit_camera_view.target().x,
            edit_camera_view.target().y,
            self.current_layer as f32,
        ));
    }

    fn update_mouse_grid_pos(&mut self, camera: &Camera, edit_camera_view: &EditCameraView) {
        let p = self.mouse_window_pos;
        let p_near = camera.unproject(&na::Point3::new(p.x, p.y, -1.0));
        let p_far = camera.unproject(&na::Point3::new(p.x, p.y, 1.0));

        let ray = Ray {
            origin: edit_camera_view.eye(),
            velocity: p_far - p_near,
        };
        let quad = Plane {
            origin: na::Point3::new(
                0.0,
                0.0,
                self.current_layer as f32,
            ),
            direction_a: self.machine.size().x as f32 * na::Vector3::x(),
            direction_b: self.machine.size().y as f32 * na::Vector3::y(),
        };

        let intersection = ray_quad_intersection(&ray, &quad);
        self.mouse_grid_pos =
            if let Some((ray_t, _plane_pos)) = intersection {
                let ray_pos = ray.origin + ray_t * ray.velocity;
                let grid_pos = grid::Point3::new(
                    ray_pos.x.floor() as isize,
                    ray_pos.y.floor() as isize,
                    self.current_layer,
                );

                if self.machine.is_valid_pos(&grid_pos) {
                    Some(grid_pos)
                } else {
                    None
                }
            } else {
                None
            };
    }

    fn update_input(&mut self) {
        // TODO: Only perform edits if something would actually change

        if self.left_mouse_button_pressed {
            if let Some(mouse_grid_pos) = self.mouse_grid_pos {
                let edit = Edit::SetBlock(
                    mouse_grid_pos,
                    Some(self.place_block.clone())
                );
                self.run_edit(edit);
            }
        }

        if self.right_mouse_button_pressed {
            if let Some(mouse_grid_pos) = self.mouse_grid_pos {
                let edit = Edit::SetBlock(
                    mouse_grid_pos,
                    None,
                );
                self.run_edit(edit);
            }
        }
    }

    pub fn on_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::CursorMoved {
                device_id: _,
                position,
                modifiers: _,
            } => {
                self.mouse_window_pos = na::Point2::new(
                    position.x as f32,
                    position.y as f32,
                );
            }
            WindowEvent::KeyboardInput { device_id: _, input } =>
                self.on_keyboard_input(*input),
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
                modifiers,
            } =>
                self.on_mouse_input(*state, *button, *modifiers),

            _ => ()
        }
    }

    fn on_keyboard_input(&mut self, input: glutin::KeyboardInput) {
        if input.state == glutin::ElementState::Pressed {
            if let Some(keycode) = input.virtual_keycode {
                self.on_key_press(keycode);
            }
        }
    }

    fn on_key_press(&mut self, keycode: VirtualKeyCode) {
        if keycode == self.config.rotate_block_key {
            self.place_block.dir_xy = self.place_block.dir_xy.rotated_cw();
        }

        if let Some(block) = self.config.block_keys.get(&keycode) {
            self.place_block.block = block.clone();
        }

        if let Some(&layer) = self.config.layer_keys.get(&keycode) {
            if self.machine.is_valid_layer(layer) {
                self.current_layer = layer;
            }
        }
    }

    fn on_mouse_input(
        &mut self,
        state: glutin::ElementState,
        button: glutin::MouseButton,
        _modifiers: glutin::ModifiersState,
    ) {
        match button {
            glutin::MouseButton::Left => 
                self.left_mouse_button_pressed = state == glutin::ElementState::Pressed,
            glutin::MouseButton::Right =>
                self.right_mouse_button_pressed = state == glutin::ElementState::Pressed,
            _ => (),
        }
    }

    pub fn render(&mut self, out: &mut RenderLists) -> Result<(), glium::DrawError> {
        let grid_size: na::Vector3<f32> = na::convert(self.machine.size());
        render::machine::render_cuboid_wireframe(
            &render::machine::Cuboid {
                center: na::Point3::from(grid_size / 2.0),
                size: grid_size,
            },
            0.1,
            &na::Vector4::new(1.0, 1.0, 1.0, 1.0),
            &mut out.solid,
        );

        render::machine::render_machine(&self.machine, out);
        render::machine::render_xy_grid(
            &self.machine.size(),
            self.current_layer as f32 + 0.01,
            &mut out.solid,
        );

        if let Some(mouse_grid_pos) = self.mouse_grid_pos {
            assert!(self.machine.is_valid_pos(&mouse_grid_pos));

            let mouse_grid_pos_float: na::Point3<f32> = na::convert(mouse_grid_pos);

            render::machine::render_cuboid_wireframe(
                &render::machine::Cuboid {
                    center: mouse_grid_pos_float + na::Vector3::new(0.5, 0.5, 0.51),
                    size: na::Vector3::new(1.0, 1.0, 1.0),
                },
                0.015,
                &na::Vector4::new(0.9, 0.9, 0.9, 1.0),
                &mut out.solid,
            );

            let block_transform = render::machine::placed_block_transform(
                &mouse_grid_pos,
                &self.place_block.dir_xy,
            );
            render::machine::render_block(
                &self.place_block.block, 
                &block_transform,
                Some(&na::Vector4::new(0.3, 0.5, 0.9, 0.7)),
                &mut out.transparent,
            );
        }
        
        Ok(())
    }
}