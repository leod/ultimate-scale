mod action;
mod render;
mod ui;

use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::path::Path;
use std::time::Duration;

use coarse_prof::profile;
use log::{info, warn};
use nalgebra as na;

use glium::glutin::{self, MouseButton, WindowEvent};

use rendology::Camera;

use crate::edit_camera_view::EditCameraView;
use crate::input_state::InputState;
use crate::machine::grid;
use crate::machine::{Block, Machine, PlacedBlock, SavedMachine};

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
}

impl Editor {
    pub fn new(config: &Config, machine: Machine) -> Editor {
        Editor {
            config: config.clone(),
            machine,
            mode: Mode::new_select(),
            clipboard: None,
            undo: VecDeque::new(),
            redo: Vec::new(),
            current_layer: 0,
            mouse_grid_pos: None,
            mouse_block_pos: None,
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
        // TODO: Maintain current rotation when switching to a different block
        // to place.
        let piece = Piece::new_origin_block(PlacedBlock { block });

        self.mode = Mode::PlacePiece { piece };
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
        let mut edit = None;

        self.mode = match self.mode.clone() {
            Mode::SelectClickedOnBlock {
                selection,
                dragged_grid_pos,
                dragged_block_pos,
            } if input_state.is_button_pressed(MouseButton::Left) => {
                // User has clicked on a selected block. Activate drag and
                // drop as soon as the mouse grid pos changes.
                if self
                    .mouse_grid_pos
                    .map(|p| p != dragged_grid_pos)
                    .unwrap_or(false)
                {
                    let mut piece =
                        Piece::new_from_selection(&self.machine, selection.iter().cloned());

                    // Move blocks so that the block that the user clicked on is at the origin.
                    piece.shift(&(-dragged_block_pos.coords));

                    // Consider the case that we are selecting a block in layer
                    // 1 while the placement layer is at 0. Then the block would
                    // immediately be dragged into layer 0, which is
                    // undesirable. Thus, we calculate a `layer_offset` here,
                    // which is subtracted from the piece z coords.
                    let layer_offset = dragged_block_pos.z - self.current_layer as isize;
                    piece.shift(&(grid::Vector3::z() * layer_offset));

                    Mode::DragAndDrop { selection, piece }
                } else {
                    // Mouse grid position has not changed (yet?).
                    Mode::SelectClickedOnBlock {
                        selection,
                        dragged_grid_pos,
                        dragged_block_pos,
                    }
                }
            }
            Mode::SelectClickedOnBlock { selection, .. }
                if !input_state.is_button_pressed(MouseButton::Left) =>
            {
                // Stop trying to go into drag and drop mode.
                Mode::new_selection(selection)
            }
            Mode::Select { selection, .. } if input_state.is_button_pressed(MouseButton::Right) => {
                if let Some(mouse_block_pos) = self.mouse_block_pos {
                    let edit = Edit::SetBlocks(maplit::hashmap! {
                        mouse_block_pos => None,
                    });
                    self.run_and_track_edit(edit);
                }

                Mode::Select { selection }
            }
            Mode::RectSelect {
                existing_selection,
                new_selection,
                ..
            } if !input_state.is_button_pressed(MouseButton::Left) => {
                // Leave rect selection if left mouse button is no longer
                // pressed.

                // Note: We do not use the mouse button released event for
                // leaving rect select mode, since this event could be
                // dropped, e.g. when the window loses focus.
                let mut selection = existing_selection;
                for p in new_selection.iter() {
                    if !selection.contains(p) {
                        selection.push(*p);
                    }
                }

                Mode::new_selection(selection)
            }
            Mode::RectSelect {
                existing_selection,
                start_pos,
                ..
            } if input_state.is_button_pressed(MouseButton::Left) => {
                // Update selection according to rectangle
                let end_pos = input_state.mouse_window_pos();
                let new_selection =
                    pick::pick_window_rect(&self.machine, camera, &start_pos, &end_pos);

                Mode::RectSelect {
                    existing_selection,
                    new_selection: new_selection.collect(),
                    start_pos,
                    end_pos: input_state.mouse_window_pos(),
                }
            }
            Mode::PlacePiece { piece } if input_state.is_button_pressed(MouseButton::Left) => {
                if let Some(mouse_grid_pos) = self.mouse_grid_pos {
                    let mut piece = piece.clone();
                    piece.shift(&mouse_grid_pos.coords);

                    let edit = piece.as_place_edit();
                    self.run_and_track_edit(edit);
                }

                Mode::PlacePiece { piece }
            }
            Mode::PlacePiece { piece } if input_state.is_button_pressed(MouseButton::Right) => {
                if let Some(mouse_grid_pos) = self.mouse_grid_pos {
                    let edit = Edit::SetBlocks(maplit::hashmap! {
                        mouse_grid_pos => None,
                    });
                    self.run_and_track_edit(edit);
                }

                Mode::PlacePiece { piece }
            }
            Mode::DragAndDrop { selection, .. }
                if input_state.is_button_pressed(MouseButton::Right) =>
            {
                // Return to selection mode on right mouse click.
                Mode::new_selection(selection)
            }
            Mode::DragAndDrop {
                selection,
                mut piece,
            } if !input_state.is_button_pressed(MouseButton::Left) => {
                // Drop the dragged stuff.
                if let Some(mouse_grid_pos) = self.mouse_grid_pos {
                    // First remove the selected blocks.
                    let remove_edit =
                        Edit::SetBlocks(selection.iter().map(|p| (*p, None)).collect());

                    // Then place the piece at the new position.
                    piece.shift(&mouse_grid_pos.coords);
                    let place_edit = piece.as_place_edit();

                    let new_selection = piece
                        .iter()
                        .map(|(p, _)| p)
                        .filter(|p| self.machine.is_valid_pos(p))
                        .collect();

                    edit = Some(Edit::compose(remove_edit, place_edit));

                    Mode::new_selection(new_selection)
                } else {
                    // Mouse not at a grid position, Just return to selection
                    // mode.
                    Mode::new_selection(selection)
                }
            }
            Mode::PipeTool {
                last_pos: None,
                rotation_xy,
                ..
            } if input_state.is_button_pressed(MouseButton::Right) => {
                if let Some(mouse_grid_pos) = self.mouse_grid_pos {
                    let edit = Edit::SetBlocks(maplit::hashmap! {
                        mouse_grid_pos => None,
                    });
                    self.run_and_track_edit(edit);
                }

                Mode::new_pipe_tool_with_rotation(rotation_xy)
            }
            Mode::PipeTool { rotation_xy, .. }
                if input_state.is_button_pressed(MouseButton::Right) =>
            {
                // Abort placement.
                Mode::new_pipe_tool_with_rotation(rotation_xy)
            }
            Mode::PipeTool {
                rotation_xy,
                blocks,
                ..
            } if !input_state.is_button_pressed(MouseButton::Left) => {
                // Finish placement.
                edit = Some(Edit::SetBlocks(
                    blocks
                        .iter()
                        .map(|(pos, block)| (*pos, Some(block.clone())))
                        .collect(),
                ));

                Mode::new_pipe_tool_with_rotation(rotation_xy)
            }
            Mode::PipeTool {
                last_pos: Some(last_pos),
                rotation_xy,
                blocks,
                ..
            } if input_state.is_button_pressed(MouseButton::Left) => {
                // Continue in pipe tool placement mode
                self.update_input_continue_pipe_tool(last_pos, rotation_xy, blocks)
            }
            x => {
                // No mode update.
                x
            }
        };

        if let Some(edit) = edit {
            self.run_and_track_edit(edit);
        }
    }

    fn update_input_continue_pipe_tool(
        &self,
        last_pos: grid::Point3,
        rotation_xy: usize,
        mut blocks: HashMap<grid::Point3, PlacedBlock>,
    ) -> Mode {
        let mouse_grid_pos = self
            .mouse_grid_pos
            .filter(|p| self.machine.is_valid_pos(p) && last_pos != *p);

        if let Some(mouse_grid_pos) = mouse_grid_pos {
            let delta = mouse_grid_pos - last_pos;
            let delta_dir = grid::Dir3::ALL
                .iter()
                .find(|dir| dir.to_vector() == delta)
                .cloned();
            if let Some(delta_dir) = delta_dir {
                // Change the previously placed pipe so that it points to the
                // new tentative pipe
                let last_block = blocks.get(&last_pos);
                let new_block = blocks
                    .get(&mouse_grid_pos)
                    .map_or_else(|| self.machine.get(&mouse_grid_pos), |block| Some(block))
                    .cloned()
                    .unwrap_or_else(|| PlacedBlock {
                        block: Block::GeneralPipe(grid::DirMap3::from_fn(|_| false)),
                    });

                let connect = last_block.map_or(true, |last_block| {
                    let last_is_pipe = if let Block::GeneralPipe(_) = last_block.block {
                        true
                    } else {
                        false
                    };
                    let new_is_pipe = if let Block::GeneralPipe(_) = new_block.block {
                        true
                    } else {
                        false
                    };

                    let connect_last = last_is_pipe || last_block.block.has_wind_hole(delta_dir);
                    let connect_new =
                        new_is_pipe || new_block.block.has_wind_hole(delta_dir.invert());

                    connect_last && connect_new
                });

                if connect {
                    if let Some(last_block) = last_block {
                        let updated_last_block =
                            self.pipe_tool_connect_pipe(&blocks, last_block, &last_pos, delta_dir);
                        blocks.insert(last_pos, updated_last_block);
                    }

                    let updated_new_block = self.pipe_tool_connect_pipe(
                        &blocks,
                        &new_block,
                        &mouse_grid_pos,
                        delta_dir.invert(),
                    );
                    blocks.insert(mouse_grid_pos, updated_new_block);
                } else {
                    blocks.insert(mouse_grid_pos, new_block);
                }
            } else {
                // New mouse grid position is not a neighbor of last_pos
                let mut block = Block::GeneralPipe(grid::DirMap3::from_fn(|dir| {
                    dir == grid::Dir3::Y_NEG || dir == grid::Dir3::Y_POS
                }));
                for _ in 0..rotation_xy {
                    block.mutate_dirs(|dir| dir.rotated_cw_xy());
                }

                blocks.insert(mouse_grid_pos, PlacedBlock { block });
            }

            Mode::PipeTool {
                last_pos: Some(mouse_grid_pos),
                rotation_xy,
                blocks,
            }
        } else {
            // No change
            Mode::PipeTool {
                last_pos: Some(last_pos),
                rotation_xy,
                blocks,
            }
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
        } else if key == self.config.select_all_key {
            self.action_select_all();
        } else if key == self.config.select_key {
            self.action_select_mode();
        } else if key == self.config.pipe_tool_key {
            self.action_pipe_tool_mode();
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
        } else if key == self.config.mirror_y_key {
            self.action_mirror_y();
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
            Mode::Select { selection, .. }
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
                    // Don't overwrite existing block when starting placement
                    let placed_block = self.machine.get(&mouse_grid_pos).map_or_else(
                        || {
                            let mut block = Block::GeneralPipe(grid::DirMap3::from_fn(|dir| {
                                dir == grid::Dir3::Y_NEG || dir == grid::Dir3::Y_POS
                            }));
                            for _ in 0..rotation_xy {
                                block.mutate_dirs(|dir| dir.rotated_cw_xy());
                            }
                            PlacedBlock { block }
                        },
                        |placed_block| placed_block.clone(),
                    );

                    let blocks = maplit::hashmap! { mouse_grid_pos => placed_block };

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
        let block_pos = self.mouse_block_pos.filter(|p| self.machine.is_block_at(p));

        if let Some(block_pos) = block_pos {
            // Clicked on a block!

            if modifiers.shift && selection.is_empty() {
                // Shift with an empty selection means to always go into rect select.
                let start_pos = input_state.mouse_window_pos();

                Mode::RectSelect {
                    existing_selection: Vec::new(),
                    new_selection: Vec::new(),
                    start_pos,
                    end_pos: start_pos,
                }
            } else if modifiers.shift && !selection.is_empty() {
                // Shift: Select in a line from the last to the current grid
                // position.

                // Safe to unwrap due to `is_empty()` check above.
                let last = selection.last().unwrap();

                // For now draw line only if there are two shared coordinates,
                // otherwise behavior is too wonky. Note that rust guarantees
                // bools to be either 0 or 1 when cast to integer types.
                let num_shared = (last.x == block_pos.x) as usize
                    + (last.y == block_pos.y) as usize
                    + (last.z == block_pos.z) as usize;
                let line = if num_shared == 2 {
                    pick::pick_line(&self.machine, last, &block_pos)
                } else {
                    vec![block_pos]
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
                Mode::new_selection(selection)
            } else if modifiers.ctrl {
                // Control: Extend/toggle block selection.
                if selection.contains(&block_pos) {
                    selection.retain(|p| *p != block_pos);
                } else {
                    selection.push(block_pos);
                }

                // Stay in selection mode.
                Mode::new_selection(selection)
            } else {
                // No modifier, but clicked on a block...
                if !selection.contains(&block_pos) {
                    // Different block, select only this one.
                    selection = Vec::new();
                    selection.push(block_pos);
                }

                if let Some(grid_pos) = self.mouse_grid_pos {
                    // Remember clicked mouse pos to allow switching to drag and
                    // drop mode as soon as the grid position changes.
                    Mode::SelectClickedOnBlock {
                        selection,
                        dragged_block_pos: block_pos,
                        dragged_grid_pos: grid_pos,
                    }
                } else {
                    // Stay in selection mode.
                    Mode::new_selection(selection)
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

    fn pipe_tool_connect_pipe(
        &self,
        blocks: &HashMap<grid::Point3, PlacedBlock>,
        placed_block: &PlacedBlock,
        block_pos: &grid::Point3,
        new_dir: grid::Dir3,
    ) -> PlacedBlock {
        match placed_block.block {
            Block::Pipe(dir_a, dir_b) => {
                let is_connected = |pos: grid::Point3, dir: grid::Dir3| {
                    let tentative = blocks
                        .get(&(pos + dir.to_vector()))
                        .map_or(false, |neighbor| neighbor.block.has_wind_hole(dir.invert()));
                    let existing = self
                        .machine
                        .get(&(pos + dir.to_vector()))
                        .map_or(false, |neighbor| neighbor.block.has_wind_hole(dir.invert()));

                    placed_block.block.has_wind_hole(dir) && (tentative || existing)
                };

                let is_a_connected = is_connected(*block_pos, dir_a);
                let is_b_connected = is_connected(*block_pos, dir_b);

                let block = if dir_a == new_dir || dir_b == new_dir {
                    // Don't need to change the existing pipe
                    Block::Pipe(dir_a, dir_b)
                } else if !is_a_connected && dir_b != new_dir {
                    Block::Pipe(new_dir, dir_b)
                } else if !is_b_connected && dir_a != new_dir {
                    Block::Pipe(dir_a, new_dir)
                } else if dir_a.0 != grid::Axis3::Z
                    && dir_b.0 != grid::Axis3::Z
                    && new_dir.0 != grid::Axis3::Z
                {
                    Block::PipeMergeXY
                } else {
                    // No way to connect previously placed pipe
                    Block::Pipe(dir_a, dir_b)
                };

                PlacedBlock { block }
            }
            Block::GeneralPipe(ref dirs) => {
                let mut new_dirs = dirs.clone();
                new_dirs[new_dir] = true;

                let block = Block::GeneralPipe(new_dirs);

                PlacedBlock { block }
            }
            _ => placed_block.clone(),
        }
    }
}
