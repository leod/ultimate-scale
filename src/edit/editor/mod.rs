mod action;
mod render;
mod ui;

use std::collections::VecDeque;
use std::fs::File;
use std::path::Path;
use std::time::Duration;

use log::{info, warn};

use nalgebra as na;

use glium::glutin::{self, MouseButton, WindowEvent};

use crate::input_state::InputState;
use crate::machine::grid;
use crate::machine::{Block, Machine, PlacedBlock, SavedMachine};
use crate::render::{Camera, EditCameraView};

use crate::edit::config::ModifiedKey;
use crate::edit::{pick, Config, Edit, Mode, Piece};

/// Maximal length of the undo queue.
pub const MAX_UNDOS: usize = 1000;

pub struct Editor {
    /// Configuration for the editor, e.g. shortcuts.
    config: Config,

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
    /// coordinate is always set to `current_layer`. Note that the grid
    /// position may point outside of the grid.
    mouse_grid_pos: Option<grid::Point3>,

    /// Position of the *block* the mouse is currently pointing to, if any.
    mouse_block_pos: Option<grid::Point3>,

    /// We keep track of the window size for fixing window positions in the UI.
    window_size: na::Vector2<f32>,
}

impl Editor {
    pub fn new(config: &Config, machine: Machine) -> Editor {
        Editor {
            config: config.clone(),
            machine,
            mode: Mode::Select(Vec::new()),
            clipboard: None,
            undo: VecDeque::new(),
            redo: Vec::new(),
            current_layer: 0,
            mouse_grid_pos: None,
            mouse_block_pos: None,
            window_size: na::Vector2::zeros(),
        }
    }

    pub fn machine(&self) -> &Machine {
        &self.machine
    }

    pub fn run_edit(&mut self, edit: Edit) -> Edit {
        let undo_edit = edit.run(&mut self.machine);

        // Now that the machine has been mutated, we need to make sure there is
        // no spurious state left in the editing mode.
        // TODO: use take_mut or mem::replace
        self.mode = self
            .mode
            .clone()
            .make_consistent_with_machine(&self.machine);

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

    pub fn switch_to_place_block_mode(&mut self, block: Block) {
        let placed_block = PlacedBlock {
            rotation_xy: 0,
            block,
        };

        let piece = match &self.mode {
            Mode::PlacePiece { piece, .. } => {
                // TODO: Maintain current rotation when switching to a
                // different block to place.
                Piece::new_origin_block(placed_block)
            }
            _ => Piece::new_origin_block(placed_block),
        };

        self.mode = Mode::PlacePiece {
            piece,
            offset: grid::Vector3::zeros(),
        };
    }

    pub fn update(
        &mut self,
        _dt: Duration,
        input_state: &InputState,
        camera: &Camera,
        edit_camera_view: &mut EditCameraView,
    ) {
        profile!("editor");

        edit_camera_view.set_target(na::Point3::new(
            edit_camera_view.target().x,
            edit_camera_view.target().y,
            self.current_layer as f32,
        ));

        self.window_size = na::Vector2::new(camera.viewport.z, camera.viewport.w);

        self.mouse_grid_pos = pick::pick_in_layer_plane(
            &self.machine,
            self.current_layer,
            camera,
            &edit_camera_view.eye(),
            &input_state.mouse_window_pos(),
        );
        self.mouse_block_pos = pick::pick_block(
            &self.machine,
            camera,
            &edit_camera_view.eye(),
            &input_state.mouse_window_pos(),
        );

        self.update_input(input_state, camera);
    }

    fn update_input(&mut self, input_state: &InputState, camera: &Camera) {
        let mut new_mode = None;
        let mut edit = None;

        match &self.mode {
            Mode::Select(_selection) => {
                // Nothing here for now.
            }
            Mode::RectSelect {
                existing_selection,
                new_selection,
                start_pos,
                end_pos: _,
            } => {
                if !input_state.is_button_pressed(MouseButton::Left) {
                    // Note: We do not use the mouse button released event for
                    // leaving rect select mode, since this event could be
                    // dropped, e.g. when the window loses focus.
                    let mut selection = existing_selection.clone();
                    for p in new_selection.iter() {
                        if !selection.contains(p) {
                            selection.push(*p);
                        }
                    }
                    new_mode = Some(Mode::Select(selection));
                } else {
                    // TODO: Could move here, but wouldn't be fun I guess
                    let end_pos = input_state.mouse_window_pos();
                    let new_selection =
                        pick::pick_window_rect(&self.machine, camera, start_pos, &end_pos);

                    new_mode = Some(Mode::RectSelect {
                        existing_selection: existing_selection.clone(),
                        new_selection: new_selection.collect(),
                        start_pos: *start_pos,
                        end_pos: input_state.mouse_window_pos(),
                    });
                }
            }
            Mode::PlacePiece { piece, offset } => {
                if input_state.is_button_pressed(MouseButton::Left) {
                    if let Some(mouse_grid_pos) = self.mouse_grid_pos {
                        let edit = piece.place_edit(&(mouse_grid_pos.coords + offset));
                        self.run_and_track_edit(edit);
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
            Mode::DragAndDrop {
                selection,
                center_pos,
                rotation_xy,
                layer_offset,
            } => {
                if !input_state.is_button_pressed(MouseButton::Left) {
                    if let Some(mouse_grid_pos) = self.mouse_grid_pos {
                        let (piece, center_pos_transformed) = self
                            .drag_and_drop_piece_from_selection(
                                selection,
                                center_pos,
                                *rotation_xy,
                                *layer_offset,
                            );
                        let offset = mouse_grid_pos - center_pos_transformed;

                        // First remove the selected blocks.
                        let remove_edit =
                            Edit::SetBlocks(selection.iter().map(|p| (*p, None)).collect());

                        // Then place the piece at the new position.
                        let place_edit = piece.place_edit(&offset);

                        let new_selection = piece
                            .iter_blocks(&offset)
                            .map(|(p, _)| p)
                            .filter(|p| self.machine.is_valid_pos(p))
                            .collect();

                        edit = Some(Edit::compose(remove_edit, place_edit));
                        new_mode = Some(Mode::Select(new_selection));
                    }
                }

                if input_state.is_button_pressed(MouseButton::Right) {
                    new_mode = Some(Mode::Select(selection.clone()));
                }
            }
            Mode::PipeTool {
                last_pos,
                rotation_xy,
                blocks,
            } => {
                // TODO

                if input_state.is_button_pressed(MouseButton::Right) {
                    // Abort placement
                    new_mode = Some(Mode::new_pipe_tool_with_rotation(*rotation_xy));
                } else if !input_state.is_button_pressed(MouseButton::Left) {
                    if last_pos.is_some() {
                        // Finish placement
                        edit = Some(Edit::SetBlocks(
                            blocks
                                .iter()
                                .map(|(pos, block)| (*pos, Some(block.clone())))
                                .collect(),
                        ));
                    }

                    new_mode = Some(Mode::new_pipe_tool_with_rotation(*rotation_xy));
                } else if last_pos.is_some() {
                    // Continue in placement mode
                    let mouse_grid_pos =
                        self.mouse_grid_pos.filter(|p| self.machine.is_valid_pos(p));

                    if let Some(mouse_grid_pos) = mouse_grid_pos {
                        let mut blocks = blocks.clone();
                        blocks.insert(
                            mouse_grid_pos,
                            PlacedBlock {
                                rotation_xy: *rotation_xy,
                                block: Block::Pipe(grid::Dir3::Y_NEG, grid::Dir3::Y_POS),
                            },
                        );

                        new_mode = Some(Mode::PipeTool {
                            last_pos: Some(mouse_grid_pos),
                            rotation_xy: *rotation_xy,
                            blocks,
                        });
                    }
                }
            }
        }

        if let Some(new_mode) = new_mode {
            self.mode = new_mode;
        }

        if let Some(edit) = edit {
            self.run_and_track_edit(edit);
        }
    }

    pub fn on_event(&mut self, input_state: &InputState, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { input, .. } => self.on_keyboard_input(input_state, input),
            WindowEvent::MouseInput {
                state,
                button,
                modifiers,
                ..
            } => self.on_mouse_input(input_state, *state, *button, *modifiers),

            _ => (),
        }
    }

    fn on_keyboard_input(&mut self, _input_state: &InputState, input: &glutin::KeyboardInput) {
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
        // Action shortcuts
        if key == self.config.undo_key {
            self.action_undo();
        } else if key == self.config.redo_key {
            self.action_redo();
        } else if key == self.config.paste_key {
            self.action_paste();
        } else if key == self.config.save_key {
            self.action_save();
        } else if key == self.config.layer_up_key {
            self.action_layer_up();
        } else if key == self.config.layer_down_key {
            self.action_layer_down();
        } else if key == self.config.select_key {
            self.action_select_mode();
        } else if key == self.config.cancel_key {
            self.action_cancel();
        } else if key == self.config.cut_key {
            self.action_cut();
        } else if key == self.config.copy_key {
            self.action_copy();
        } else if key == self.config.delete_key {
            self.action_delete();
        } else if key == self.config.block_kind_key {
            self.action_next_kind();
        } else if key == self.config.rotate_block_cw_key {
            self.action_rotate_cw();
        } else if key == self.config.rotate_block_ccw_key {
            self.action_rotate_ccw();
        }

        // Switch to specific layer
        if let Some((_key, layer)) = self
            .config
            .layer_keys
            .iter()
            .find(|(layer_key, _layer)| key == *layer_key)
        {
            if self.machine.is_valid_layer(*layer) {
                self.current_layer = *layer;
            }
        }

        // Switch to specific place block mode
        if let Some((_key, block)) = self
            .config
            .block_keys
            .iter()
            .cloned()
            .find(|(block_key, _block)| key == *block_key)
        {
            self.switch_to_place_block_mode(block);
        }
    }

    fn on_mouse_input(
        &mut self,
        input_state: &InputState,
        state: glutin::ElementState,
        button: glutin::MouseButton,
        modifiers: glutin::ModifiersState,
    ) {
        self.mode = match self.mode.clone() {
            Mode::Select(selection)
                if button == glutin::MouseButton::Left
                    && state == glutin::ElementState::Pressed =>
            {
                self.on_left_mouse_click_select(input_state, modifiers, selection)
            }
            Mode::PipeTool { rotation_xy, .. }
                if button == glutin::MouseButton::Left
                    && state == glutin::ElementState::Pressed =>
            {
                // Start placement?
                let mouse_grid_pos = self.mouse_grid_pos.filter(|p| self.machine.is_valid_pos(p));

                if let Some(mouse_grid_pos) = mouse_grid_pos {
                    let blocks = maplit::hashmap! {
                        mouse_grid_pos => PlacedBlock {
                            rotation_xy,
                            block: Block::Pipe(grid::Dir3::Y_NEG, grid::Dir3::Y_POS),
                        },
                    };

                    Mode::PipeTool {
                        last_pos: Some(mouse_grid_pos),
                        rotation_xy,
                        blocks,
                    }
                } else {
                    Mode::new_pipe_tool_with_rotation(rotation_xy)
                }
            }
            x => x,
        }
    }

    fn on_left_mouse_click_select(
        &self,
        input_state: &InputState,
        modifiers: glutin::ModifiersState,
        mut selection: Vec<grid::Point3>,
    ) -> Mode {
        // Double check that there actually is a block at the mouse block
        // position.
        let grid_pos = self
            .mouse_block_pos
            .filter(|p| self.machine.get_block_at_pos(p).is_some());

        if let Some(grid_pos) = grid_pos {
            // Clicked on a block!

            if modifiers.shift && !selection.is_empty() {
                // Shift: Select in a line from the last to the current grid
                // position.

                // Safe to unwrap due to `is_empty()` check above.
                let last = selection.last().unwrap();

                // For now draw line only if there are two shared coordinates,
                // otherwise behavior is too wonky. Note that rust guarantees
                // bools to be either 0 or 1 when cast to integer types.
                let num_shared = (last.x == grid_pos.x) as usize
                    + (last.y == grid_pos.y) as usize
                    + (last.z == grid_pos.z) as usize;
                let line = if num_shared == 2 {
                    pick::pick_line(&self.machine, last, &grid_pos)
                } else {
                    vec![grid_pos]
                };

                // Push the selected line to the end of the vector, so that it
                // counts as the most recently selected.
                selection.retain(|p| !line.contains(p));

                if !modifiers.ctrl {
                    for p in line {
                        selection.push(p);
                    }
                }

                // Stay in selection mode.
                Mode::Select(selection)
            } else if modifiers.ctrl {
                // Control: Extend/toggle block selection.
                if selection.contains(&grid_pos) {
                    selection.retain(|p| *p != grid_pos);
                } else {
                    selection.push(grid_pos);
                }

                // Stay in selection mode.
                Mode::Select(selection)
            } else {
                // No modifier, but clicked on a block...
                if !selection.contains(&grid_pos) {
                    // Different block, select only this one.
                    selection = Vec::new();
                    selection.push(grid_pos);
                }

                // Consider the case that we are selecting a block in layer 1
                // while the placement layer is at 0. Then the block would
                // immediately be dragged into layer 0, which is undesirable.
                // Thus, we calculate a `layer_offset` here, which is
                // subtracted from the piece z coords before placing.
                let layer_offset = grid_pos.z - self.current_layer as isize;

                Mode::DragAndDrop {
                    selection,
                    center_pos: grid_pos,
                    rotation_xy: 0,
                    layer_offset,
                }
            }
        } else {
            // Did not click on a block, switch to rect select.
            let existing_selection = if modifiers.ctrl {
                // Control: Keep existing selection.
                selection
            } else {
                // Start from scratch otherwise.
                Vec::new()
            };

            let start_pos = input_state.mouse_window_pos();

            Mode::RectSelect {
                existing_selection,
                new_selection: Vec::new(),
                start_pos,
                end_pos: start_pos,
            }
        }
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

    fn drag_and_drop_piece_from_selection(
        &self,
        selection: &[grid::Point3],
        center_pos: &grid::Point3,
        rotation_xy: usize,
        layer_offset: isize,
    ) -> (Piece, grid::Point3) {
        let selected_blocks =
            Piece::selected_blocks(&self.machine, selection.iter().cloned()).collect::<Vec<_>>();
        let mut piece = Piece::new_blocks_to_origin(&selected_blocks);
        for _ in 0..rotation_xy {
            piece.rotate_cw_xy();
        }

        // Get the `center_pos` after it was transformed by centering and
        // rotation.
        let center_pos_index = selected_blocks
            .iter()
            .position(|(p, _)| p == center_pos)
            .expect("Mode::DragAndDrop must always contain center_pos in selection");
        let mut center_pos_transformed = piece.block_at_index(center_pos_index).0;
        center_pos_transformed.z -= layer_offset;

        (piece, center_pos_transformed)
    }
}
