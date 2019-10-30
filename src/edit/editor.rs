use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::path::Path;
use std::time::Duration;

use log::{info, warn};

use nalgebra as na;

use glium::glutin::{self, MouseButton, WindowEvent};

use crate::exec::{self, ExecView};
use crate::input_state::InputState;
use crate::machine::grid;
use crate::machine::{Block, Machine, PlacedBlock, SavedMachine};
use crate::render::pipeline::RenderLists;
use crate::render::{self, Camera, EditCameraView};

use crate::edit::config::ModifiedKey;
use crate::edit::{pick, Config, Edit, Mode, Piece};

/// Maximal length of the undo queue.
pub const MAX_UNDOS: usize = 1000;

pub struct Editor {
    /// Configuration for the editor, e.g. shortcuts.
    config: Config,

    /// Configuration for running a machine.
    exec_config: exec::view::Config,

    /// The machine being edited.
    machine: Machine,

    /// The current editing mode.
    mode: Mode,

    /// Clipboard.
    clipboard: Option<Piece>,

    /// Edits that undo the last performed edits, in the order that the edits
    /// were performed.
    undo: VecDeque<Edit>,

    /// Edits that redo the last performed undos, in the order that the undos
    /// were performed.
    redo: Vec<Edit>,

    /// Layer being edited. Blocks are placed only in the current layer.
    current_layer: isize,

    /// Grid position the mouse is currently pointing to, if any. The z
    /// coordinate is always set to `current_layer`.
    mouse_grid_pos: Option<grid::Point3>,

    /// Whether to start executing the machine in the next `update` call.
    start_exec: bool,

    /// We keep track of the window size for fixing window positions in the UI.
    window_size: na::Vector2<f32>,
}

impl Editor {
    pub fn new(config: &Config, exec_config: &exec::view::Config, machine: Machine) -> Editor {
        Editor {
            config: config.clone(),
            exec_config: exec_config.clone(),
            machine,
            mode: Mode::PlacePiece(Piece::new_origin_block(PlacedBlock {
                rotation_xy: 1,
                block: Block::PipeXY,
            })),
            clipboard: None,
            undo: VecDeque::new(),
            redo: Vec::new(),
            current_layer: 0,
            mouse_grid_pos: None,
            start_exec: false,
            window_size: na::Vector2::zeros(),
        }
    }

    pub fn machine(&self) -> &Machine {
        &self.machine
    }

    pub fn run_edit(&mut self, edit: Edit) -> Edit {
        let undo_edit = edit.run(&mut self.machine);

        // Make sure our state is in track with the edited machine
        self.mode = match self.mode.clone() {
            Mode::Select(mut selection) => {
                selection.retain(|grid_pos| self.machine.get_block_at_pos(grid_pos).is_some());
                Mode::Select(selection)
            }
            mode => mode,
        };

        undo_edit
    }

    pub fn run_and_track_edit(&mut self, edit: Edit) {
        let undo_edit = self.run_edit(edit);

        match undo_edit {
            Edit::NoOp => {
                // Don't pollute undo queue with edits that do nothing
            }
            undo_edit => {
                self.undo.push_back(undo_edit);
                if self.undo.len() > MAX_UNDOS {
                    self.undo.pop_front();
                }

                self.redo.clear();
            }
        }
    }

    pub fn undo_last_edit(&mut self) {
        if let Some(undo_edit) = self.undo.pop_back() {
            let redo_edit = self.run_edit(undo_edit);
            self.redo.push(redo_edit);
        }
    }

    pub fn redo_last_edit(&mut self) {
        if let Some(redo_edit) = self.redo.pop() {
            let undo_edit = self.run_edit(redo_edit);
            self.undo.push_back(undo_edit);
        }
    }

    pub fn switch_to_place_block_mode(&mut self, block: Block) {
        let placed_block = PlacedBlock {
            rotation_xy: 0,
            block,
        };

        self.mode = Mode::PlacePiece(match &self.mode {
            Mode::PlacePiece(piece) => {
                // TODO: Maintain current rotation when switching to a
                // different block to place.
                Piece::new_origin_block(placed_block)
            }
            _ => Piece::new_origin_block(placed_block),
        });
    }

    pub fn update(
        &mut self,
        _dt: Duration,
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

        self.mouse_grid_pos = pick::pick_in_layer(
            &self.machine,
            self.current_layer,
            camera,
            &edit_camera_view.eye(),
            &input_state.mouse_window_pos(),
        );

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
        let bg_alpha = 0.8;

        imgui::Window::new(imgui::im_str!("Blocks"))
            .horizontal_scrollbar(true)
            .movable(false)
            .always_auto_resize(true)
            .position([self.window_size.x - 10.0, 10.0], imgui::Condition::Always)
            .position_pivot([1.0, 0.0])
            .bg_alpha(bg_alpha)
            .build(&ui, || {
                for (block_key, block) in self.config.block_keys.clone().iter() {
                    if ui.button(
                        &imgui::ImString::new(block.name()),
                        [blocks_width - 20.0, 40.0],
                    ) {
                        self.switch_to_place_block_mode(*block);
                    }

                    if ui.is_item_hovered() {
                        let text = format!("{}\nShortcut: {}", block.description(), block_key);
                        ui.tooltip(|| ui.text(&imgui::ImString::new(text)));
                    }
                }
            });

        imgui::Window::new(imgui::im_str!("Tools"))
            .horizontal_scrollbar(true)
            .movable(false)
            .always_auto_resize(true)
            .position([10.0, 10.0], imgui::Condition::Always)
            .bg_alpha(bg_alpha)
            .build(&ui, || {
                if ui.button(imgui::im_str!("Select"), [blocks_width - 20.0, 40.0]) {
                    self.mode = Mode::Select(HashSet::new());
                }

                if ui.is_item_hovered() {
                    let text = format!("Shortcut: TODO");
                    ui.tooltip(|| ui.text(&imgui::ImString::new(text)));
                }
            });
    }

    fn update_input(&mut self, input_state: &InputState) {
        match &self.mode {
            Mode::Select(_selection) => {
                // TODO
            }
            Mode::PlacePiece(piece) => {
                if input_state.is_button_pressed(MouseButton::Left) {
                    if let Some(mouse_grid_pos) = self.mouse_grid_pos {
                        self.run_and_track_edit(piece.place_edit(&mouse_grid_pos.coords));
                    }
                }

                if input_state.is_button_pressed(MouseButton::Right) {
                    if let Some(mouse_grid_pos) = self.mouse_grid_pos {
                        let edit = Edit::SetBlocks(maplit::hashmap! {
                            mouse_grid_pos => None,
                        });
                        self.run_and_track_edit(edit);
                    }
                }
            }
        }
    }

    pub fn on_event(&mut self, event: &WindowEvent) {
        match event {
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
        let edit = match &mut self.mode {
            Mode::Select(selection) => {
                if key == self.config.cut_key {
                    let edit = Edit::SetBlocks(selection.iter().map(|p| (*p, None)).collect());

                    let mut selected_blocks = HashMap::new();
                    for p in selection.iter() {
                        if let Some((_, placed_block)) = self.machine.get_block_at_pos(p) {
                            selected_blocks.insert(*p, placed_block.clone());
                        }
                    }
                    self.clipboard = Some(Piece::new_blocks_to_origin(selected_blocks));

                    Some(edit)
                } else if key == self.config.copy_key {
                    let mut selected_blocks = HashMap::new();
                    for p in selection.iter() {
                        if let Some((_, placed_block)) = self.machine.get_block_at_pos(p) {
                            selected_blocks.insert(*p, placed_block.clone());
                        }
                    }
                    self.clipboard = Some(Piece::new_blocks_to_origin(selected_blocks));

                    None
                } else {
                    None
                }
            }
            Mode::PlacePiece(piece) => {
                if key.key == self.config.rotate_block_key.key {
                    if !key.shift {
                        piece.rotate_cw_xy();
                    } else {
                        piece.rotate_ccw_xy();
                    }
                } else if key == self.config.block_kind_key {
                    /*if let Some(current_kind) = placed_block.block.kind() {
                        placed_block.block = placed_block.block.with_kind(current_kind.next());
                    }*/
                    // TODO: Switching kind after piece update
                }

                None
            }
        };

        if let Some(edit) = edit {
            self.run_and_track_edit(edit);
        }

        if key == self.config.undo_key {
            self.undo_last_edit();
        } else if key == self.config.redo_key {
            self.redo_last_edit();
        } else if key == self.config.paste_key && self.clipboard.is_some() {
            if let Some(clipboard) = &self.clipboard {
                self.mode = Mode::PlacePiece(clipboard.clone());
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
        } else if let Some((_key, layer)) = self
            .config
            .layer_keys
            .iter()
            .find(|(layer_key, _layer)| key == *layer_key)
        {
            if self.machine.is_valid_layer(*layer) {
                self.current_layer = *layer;
            }
        } else if let Some((_key, block)) = self
            .config
            .block_keys
            .iter()
            .find(|(block_key, _block)| key == *block_key)
        {
            self.switch_to_place_block_mode(*block);
        }
    }

    fn on_mouse_input(
        &mut self,
        state: glutin::ElementState,
        button: glutin::MouseButton,
        modifiers: glutin::ModifiersState,
    ) {
        self.mode = match self.mode.clone() {
            Mode::Select(mut selection) => {
                if button == glutin::MouseButton::Left && state == glutin::ElementState::Pressed {
                    // TODO: Switch to rect select etc.
                    // TODO: Raycast?
                    if let Some(grid_pos) = self.mouse_grid_pos {
                        let has_block = self.machine.get_block_at_pos(&grid_pos).is_some();

                        if has_block {
                            if !modifiers.shift && !modifiers.ctrl {
                                selection = HashSet::new();
                            }
                            selection.insert(grid_pos);
                        }
                    }

                    Mode::Select(selection)
                } else {
                    Mode::Select(selection)
                }
            }
            x => x,
        }
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
            &mut out.plain,
        );

        if let Some(mouse_grid_pos) = self.mouse_grid_pos {
            assert!(self.machine.is_valid_pos(&mouse_grid_pos));

            let mouse_grid_pos_float: na::Point3<f32> = na::convert(mouse_grid_pos);

            match &self.mode {
                Mode::Select(selection) => {
                    for &grid_pos in selection.iter() {
                        let grid_pos_float: na::Point3<f32> = na::convert(grid_pos);
                        render::machine::render_cuboid_wireframe(
                            &render::machine::Cuboid {
                                center: grid_pos_float + na::Vector3::new(0.5, 0.5, 0.51),
                                size: na::Vector3::new(1.0, 1.0, 1.0),
                            },
                            0.025,
                            &na::Vector4::new(0.9, 0.9, 0.0, 1.0),
                            &mut out.plain,
                        );
                    }

                    render::machine::render_cuboid_wireframe(
                        &render::machine::Cuboid {
                            center: mouse_grid_pos_float + na::Vector3::new(0.5, 0.5, 0.51),
                            size: na::Vector3::new(1.0, 1.0, 1.0),
                        },
                        0.015,
                        &na::Vector4::new(0.9, 0.9, 0.9, 1.0),
                        &mut out.plain,
                    );
                }
                Mode::PlacePiece(piece) => {
                    for (pos, placed_block) in piece.iter_blocks(&mouse_grid_pos.coords) {
                        let block_center = render::machine::block_center(&pos);
                        let block_transform =
                            render::machine::placed_block_transform(&placed_block);
                        render::machine::render_block(
                            &placed_block,
                            0.0,
                            &None,
                            &block_center,
                            &block_transform,
                            0.8,
                            out,
                        );
                    }

                    let wire_size: na::Vector3<f32> = na::convert(piece.grid_size());
                    let wire_center = mouse_grid_pos_float + wire_size / 2.0;
                    render::machine::render_cuboid_wireframe(
                        &render::machine::Cuboid {
                            center: wire_center,
                            size: wire_size,
                        },
                        0.015,
                        &na::Vector4::new(0.9, 0.9, 0.9, 1.0),
                        &mut out.plain,
                    );
                }
            }
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
