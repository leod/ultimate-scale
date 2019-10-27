use std::fs::File;
use std::path::Path;

use log::{info, warn};

use nalgebra as na;

use glium::glutin::{self, MouseButton, WindowEvent};

use crate::exec::{self, ExecView};
use crate::input_state::InputState;
use crate::machine::grid;
use crate::machine::{Block, Machine, PlacedBlock, SavedMachine};
use crate::render::pipeline::RenderLists;
use crate::render::{self, Camera, EditCameraView};
use crate::util::intersection::{ray_quad_intersection, Plane, Ray};

use crate::edit::config::ModifiedKey;
use crate::edit::{Config, Edit};

pub struct Editor {
    config: Config,
    exec_config: exec::view::Config,

    machine: Machine,

    place_block: PlacedBlock,

    current_layer: isize,
    mouse_window_pos: na::Point2<f32>,
    mouse_grid_pos: Option<grid::Point3>,

    start_exec: bool,

    window_size: na::Vector2<f32>,
}

impl Editor {
    pub fn new(config: &Config, exec_config: &exec::view::Config, machine: Machine) -> Editor {
        Editor {
            config: config.clone(),
            exec_config: exec_config.clone(),
            machine,
            place_block: PlacedBlock {
                rotation_xy: 1,
                block: Block::PipeXY,
            },
            current_layer: 0,
            mouse_window_pos: na::Point2::origin(),
            mouse_grid_pos: None,
            start_exec: false,
            window_size: na::Vector2::zeros(),
        }
    }

    pub fn machine(&self) -> &Machine {
        &self.machine
    }

    pub fn run_edit(&mut self, edit: Edit) {
        edit.run(&mut self.machine);
    }

    pub fn update(
        &mut self,
        _dt_secs: f32,
        input_state: &InputState,
        camera: &Camera,
        edit_camera_view: &mut EditCameraView,
    ) -> Option<ExecView> {
        profile!("editor");

        edit_camera_view.set_target(na::Point3::new(
            edit_camera_view.target().x,
            edit_camera_view.target().y,
            self.current_layer as f32,
        ));

        self.window_size = na::Vector2::new(camera.viewport.z, camera.viewport.w);

        self.update_mouse_grid_pos(camera, edit_camera_view);
        self.update_input(input_state);

        if !self.start_exec {
            None
        } else {
            info!("Starting exec");

            self.start_exec = false;

            let exec_view = ExecView::new(&self.exec_config, self.machine.clone());
            Some(exec_view)
        }
    }

    pub fn ui(&mut self, ui: &imgui::Ui) {
        let blocks_width = 160.0;
        imgui::Window::new(imgui::im_str!("Blocks"))
            .flags(
                imgui::WindowFlags::HORIZONTAL_SCROLLBAR
                    | imgui::WindowFlags::NO_MOVE
                    | imgui::WindowFlags::NO_RESIZE,
            )
            .size([blocks_width, self.window_size.y], imgui::Condition::Always)
            .position(
                [self.window_size.x - blocks_width, 0.0],
                imgui::Condition::Always,
            )
            .bg_alpha(0.8)
            .build(&ui, || {
                for (block_key, block) in self.config.block_keys.iter() {
                    if ui.button(
                        &imgui::ImString::new(block.name()),
                        [blocks_width - 20.0, 40.0],
                    ) {
                        self.place_block.block = *block;
                    }
                    if ui.is_item_hovered() {
                        let text = format!("{}\nShortcut: {}", block.description(), block_key);
                        ui.tooltip(|| ui.text(&imgui::ImString::new(text)));
                    }
                }
            })
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
            origin: na::Point3::new(0.0, 0.0, self.current_layer as f32),
            direction_a: self.machine.size().x as f32 * na::Vector3::x(),
            direction_b: self.machine.size().y as f32 * na::Vector3::y(),
        };

        let intersection = ray_quad_intersection(&ray, &quad);
        self.mouse_grid_pos = if let Some((ray_t, _plane_pos)) = intersection {
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

    fn update_input(&mut self, input_state: &InputState) {
        // TODO: Only perform edits if something would actually change

        if input_state.is_button_pressed(MouseButton::Left) {
            if let Some(mouse_grid_pos) = self.mouse_grid_pos {
                let edit = Edit::SetBlock(mouse_grid_pos, Some(self.place_block.clone()));
                self.run_edit(edit);
            }
        }

        if input_state.is_button_pressed(MouseButton::Right) {
            if let Some(mouse_grid_pos) = self.mouse_grid_pos {
                let edit = Edit::SetBlock(mouse_grid_pos, None);
                self.run_edit(edit);
            }
        }
    }

    pub fn on_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_window_pos = na::Point2::new(position.x as f32, position.y as f32);
            }
            WindowEvent::KeyboardInput { input, .. } => self.on_keyboard_input(input),
            WindowEvent::MouseInput {
                state,
                button,
                modifiers,
                ..
            } => self.on_mouse_input(*state, *button, *modifiers),

            _ => (),
        }
    }

    fn on_keyboard_input(&mut self, input: &glutin::KeyboardInput) {
        if input.state == glutin::ElementState::Pressed {
            if let Some(keycode) = input.virtual_keycode {
                let modified_key = ModifiedKey {
                    shift: input.modifiers.shift,
                    ctrl: input.modifiers.ctrl,
                    key: keycode,
                };

                self.on_key_press(modified_key);
            }
        }
    }

    fn on_key_press(&mut self, key: ModifiedKey) {
        if key.key == self.config.rotate_block_key.key {
            if !key.shift {
                self.place_block.rotate_cw();
            } else {
                self.place_block.rotate_ccw();
            }
        } else if key == self.config.block_kind_key {
            if let Some(current_kind) = self.place_block.block.kind() {
                self.place_block.block = self.place_block.block.with_kind(current_kind.next());
            }
        } else if key == self.config.start_exec_key {
            self.start_exec = true;
        } else if key == self.config.save_key {
            self.save(&self.config.default_save_path);
        } else if key == self.config.layer_up_key {
            if self.machine.is_valid_layer(self.current_layer + 1) {
                self.current_layer += 1;
            }
        } else if key == self.config.layer_down_key {
            if self.machine.is_valid_layer(self.current_layer - 1) {
                self.current_layer -= 1;
            }
        } else if let Some((_key, block)) = self
            .config
            .block_keys
            .iter()
            .find(|(block_key, _block)| key == *block_key)
        {
            self.place_block.block = *block;
        } else if let Some((_key, layer)) = self
            .config
            .layer_keys
            .iter()
            .find(|(layer_key, _layer)| key == *layer_key)
        {
            if self.machine.is_valid_layer(*layer) {
                self.current_layer = *layer;
            }
        }
    }

    fn on_mouse_input(
        &mut self,
        _state: glutin::ElementState,
        _button: glutin::MouseButton,
        _modifiers: glutin::ModifiersState,
    ) {
    }

    pub fn render(&mut self, out: &mut RenderLists) -> Result<(), glium::DrawError> {
        profile!("editor");

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

        render::machine::render_machine(&self.machine, 0.0, None, out);
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
                &mut out.plain,
            );

            let block_center = render::machine::block_center(&mouse_grid_pos);
            let block_transform = render::machine::placed_block_transform(&self.place_block);
            render::machine::render_block(
                &self.place_block,
                0.0,
                &None,
                &block_center,
                &block_transform,
                0.8,
                out,
            );
        }

        Ok(())
    }

    fn save(&self, path: &Path) {
        info!("Saving current machine to file {:?}", path);

        match File::create(path) {
            Ok(file) => {
                let saved_machine = SavedMachine::from_machine(&self.machine);
                if let Err(err) = serde_json::to_writer_pretty(file, &saved_machine) {
                    warn!(
                        "Error while saving machine to file {:?}: {}",
                        path.to_str(),
                        err
                    );
                }
            }
            Err(err) => {
                warn!(
                    "Could not open file {:?} for writing: {}",
                    path.to_str(),
                    err
                );
            }
        };
    }
}
