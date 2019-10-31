use std::collections::VecDeque;
use std::fs::File;
use std::path::Path;
use std::time::Duration;

use log::{info, warn};

use nalgebra as na;

use glium::glutin::{self, MouseButton, WindowEvent};
use imgui::{im_str, ImString};

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
    /// coordinate is always set to `current_layer`. Note that the grid
    /// position may point outside of the grid.
    mouse_grid_pos: Option<grid::Point3>,

    /// Position of the *block* the mouse is currently pointing to, if any.
    mouse_block_pos: Option<grid::Point3>,

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
            mode: Mode::Select(Vec::new()),
            clipboard: None,
            undo: VecDeque::new(),
            redo: Vec::new(),
            current_layer: 0,
            mouse_grid_pos: None,
            mouse_block_pos: None,
            start_exec: false,
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
    ) -> Option<ExecView> {
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
        let button_w = 140.0;
        let button_h = 25.0;
        let small_button_w = 66.25;
        let bg_alpha = 0.8;

        imgui::Window::new(im_str!("Blocks"))
            .horizontal_scrollbar(true)
            .movable(false)
            .always_auto_resize(true)
            .position([self.window_size.x - 10.0, 10.0], imgui::Condition::Always)
            .position_pivot([1.0, 0.0])
            .bg_alpha(bg_alpha)
            .build(&ui, || {
                let cur_block = match &self.mode {
                    Mode::PlacePiece { piece, .. } => piece.get_singleton(),
                    _ => None,
                };

                for (block_key, block) in self.config.block_keys.clone().iter() {
                    let name = &ImString::new(block.name());
                    let selected = cur_block
                        .as_ref()
                        .map_or(false, |placed_block| placed_block.block == *block);
                    let selectable = imgui::Selectable::new(name).selected(selected);

                    if selectable.build(ui) {
                        self.switch_to_place_block_mode(*block);
                    }

                    if ui.is_item_hovered() {
                        let text = format!("{}\n\nShortcut: {}", block.description(), block_key);
                        ui.tooltip(|| ui.text(&ImString::new(text)));
                    }
                }
            });

        imgui::Window::new(im_str!("Tools"))
            .horizontal_scrollbar(true)
            .movable(false)
            .always_auto_resize(true)
            .position([10.0, 10.0], imgui::Condition::Always)
            .bg_alpha(bg_alpha)
            .build(&ui, || {
                if ui.button(im_str!("Select"), [button_w, button_h]) {
                    self.mode = Mode::Select(Vec::new());
                }
                if ui.is_item_hovered() {
                    let text = format!(
                        "Switch to block selection mode.\n\nShortcut: {}",
                        self.config.select_key
                    );
                    ui.tooltip(|| ui.text(&ImString::new(text)));
                }

                ui.separator();

                if ui.button(im_str!("Undo"), [small_button_w, button_h]) {
                    self.action_undo();
                }
                if ui.is_item_hovered() {
                    let text = format!("Undo the last edit.\n\nShortcut: {}", self.config.undo_key);
                    ui.tooltip(|| ui.text(&ImString::new(text)));
                }

                ui.same_line(0.0);

                if ui.button(im_str!("Redo"), [small_button_w, button_h]) {
                    self.action_redo();
                }
                if ui.is_item_hovered() {
                    let text = format!(
                        "Take back the last undo.\n\nShortcut: {}",
                        self.config.redo_key
                    );
                    ui.tooltip(|| ui.text(&ImString::new(text)));
                }

                ui.separator();

                if ui.button(im_str!("Paste"), [button_w, button_h]) {
                    self.action_paste();
                }
                if ui.is_item_hovered() {
                    let text = format!(
                        "Start placing the last copied blocks.\n\nShortcut: {}",
                        self.config.paste_key
                    );
                    ui.tooltip(|| ui.text(&ImString::new(text)));
                }

                if ui.button(im_str!("Copy"), [small_button_w, button_h]) {
                    self.action_copy();
                }
                if ui.is_item_hovered() {
                    let text = format!(
                        "Copy selected blocks.\n\nShortcut: {}",
                        self.config.copy_key
                    );
                    ui.tooltip(|| ui.text(&ImString::new(text)));
                }

                ui.same_line(0.0);

                if ui.button(im_str!("Cut"), [small_button_w, button_h]) {
                    self.action_cut();
                }
                if ui.is_item_hovered() {
                    let text = format!(
                        "Copy and remove selected blocks.\n\nShortcut: {}",
                        self.config.cut_key
                    );
                    ui.tooltip(|| ui.text(&ImString::new(text)));
                }

                ui.separator();

                ui.set_window_font_scale(1.5);
                if ui.button(im_str!("↻"), [small_button_w, button_h]) {
                    self.action_rotate_cw();
                }
                if ui.is_item_hovered() {
                    let text = format!(
                        "Rotate blocks to be placed clockwise.\n\nShortcut: {}",
                        self.config.rotate_block_cw_key
                    );
                    ui.tooltip(|| ui.text(&ImString::new(text)));
                }

                ui.same_line(0.0);

                if ui.button(im_str!("↺"), [small_button_w, button_h]) {
                    self.action_rotate_ccw();
                }
                if ui.is_item_hovered() {
                    let text = format!(
                        "Rotate blocks to be placed counterclockwise.\n\nShortcut: {}",
                        self.config.rotate_block_ccw_key
                    );
                    ui.tooltip(|| ui.text(&ImString::new(text)));
                }
                ui.set_window_font_scale(1.0);
            });
    }

    fn update_input(&mut self, input_state: &InputState, camera: &Camera) {
        let mut new_mode = None;

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
                        self.run_and_track_edit(
                            piece.place_edit(&(mouse_grid_pos.coords + offset)),
                        );
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

        if let Some(new_mode) = new_mode {
            self.mode = new_mode;
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
        } else if key == self.config.paste_key && self.clipboard.is_some() {
            self.action_paste();
        } else if key == self.config.start_exec_key {
            self.start_exec = true;
        } else if key == self.config.save_key {
            self.action_save();
        } else if key == self.config.layer_up_key {
            self.action_layer_up();
        } else if key == self.config.layer_down_key {
            self.action_layer_down();
        } else if key == self.config.select_key || key == self.config.cancel_key {
            self.action_select_mode();
        } else if key == self.config.cut_key {
            self.action_cut();
        } else if key == self.config.copy_key {
            self.action_copy();
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
            .find(|(block_key, _block)| key == *block_key)
        {
            self.switch_to_place_block_mode(*block);
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
            Mode::Select(mut selection)
                if button == glutin::MouseButton::Left
                    && state == glutin::ElementState::Pressed =>
            {
                // Double check that there actually is a block at the mouse block
                // position.
                let grid_pos = self
                    .mouse_block_pos
                    .filter(|p| self.machine.get_block_at_pos(p).is_some());

                if let Some(grid_pos) = grid_pos {
                    if modifiers.shift && !selection.is_empty() {
                        // Shift: Select in a line from the last to the current
                        // grid position.

                        // Safe to unwrap due to `is_empty()` check above.
                        let last = selection.last().unwrap();

                        // For now draw line only if there are two shared
                        // coordinates, otherwise behavior is too wonky.
                        // Note that rust guarantees bools to be either 0 or
                        // 1 when cast to integer types.
                        let num_shared = (last.x == grid_pos.x) as usize
                            + (last.y == grid_pos.y) as usize
                            + (last.z == grid_pos.z) as usize;
                        let line = if num_shared == 2 {
                            pick::pick_line(&self.machine, last, &grid_pos)
                        } else {
                            vec![grid_pos]
                        };

                        // Push the selected line to the end of the vector, so
                        // that it counts as the most recently selected.
                        selection.retain(|p| !line.contains(p));

                        if !modifiers.ctrl {
                            for p in line {
                                selection.push(p);
                            }
                        }
                    } else if modifiers.ctrl {
                        // Control: Extend/toggle block selection.
                        if selection.contains(&grid_pos) {
                            selection.retain(|p| *p != grid_pos);
                        } else {
                            selection.push(grid_pos);
                        }
                    } else {
                        // No modifier: Only select new block.
                        selection = Vec::new();
                        selection.push(grid_pos);
                    }

                    // If clicked on a block, stay in select mode.
                    Mode::Select(selection)
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

        match &self.mode {
            Mode::Select(selection) => {
                self.render_selection(selection, false, out);

                if let Some(mouse_block_pos) = self.mouse_block_pos {
                    self.render_block_wireframe(
                        &mouse_block_pos,
                        0.015,
                        &na::Vector4::new(0.9, 0.9, 0.9, 1.0),
                        out,
                    );
                }
            }
            Mode::RectSelect {
                existing_selection,
                new_selection,
                start_pos,
                end_pos,
            } => {
                self.render_selection(existing_selection, false, out);
                self.render_selection(new_selection, false, out);

                let min = na::Point2::new(start_pos.x.min(end_pos.x), start_pos.y.min(end_pos.y));
                let max = na::Point2::new(start_pos.x.max(end_pos.x), start_pos.y.max(end_pos.y));

                let rect_transform =
                    na::Matrix4::new_translation(&na::Vector3::new(min.x, min.y, 0.0))
                        * na::Matrix4::new_nonuniform_scaling(&na::Vector3::new(
                            max.x - min.x,
                            max.y - min.y,
                            1.0,
                        ));
                out.ortho.add(
                    render::Object::Quad,
                    &render::pipeline::DefaultInstanceParams {
                        transform: rect_transform,
                        color: na::Vector4::new(0.3, 0.3, 0.9, 0.3),
                        ..Default::default()
                    },
                );
            }
            Mode::PlacePiece { piece, offset } => {
                if let Some(mouse_grid_pos) = self.mouse_grid_pos {
                    self.render_piece_to_place(piece, &(mouse_grid_pos + offset), out);
                }
            }
        }

        Ok(())
    }

    fn render_selection(
        &self,
        selection: &[grid::Point3],
        highlight_last: bool,
        out: &mut RenderLists,
    ) {
        for (i, &grid_pos) in selection.iter().enumerate() {
            let color = if highlight_last && i + 1 == selection.len() {
                na::Vector4::new(0.9, 0.9, 0.0, 1.0)
            } else {
                na::Vector4::new(0.9, 0.5, 0.0, 1.0)
            };

            let grid_pos_float: na::Point3<f32> = na::convert(grid_pos);
            render::machine::render_cuboid_wireframe(
                &render::machine::Cuboid {
                    center: grid_pos_float + na::Vector3::new(0.5, 0.5, 0.51),
                    size: na::Vector3::new(1.0, 1.0, 1.0),
                },
                0.025,
                &color,
                &mut out.plain,
            );
        }
    }

    fn render_block_wireframe(
        &self,
        pos: &grid::Point3,
        thickness: f32,
        color: &na::Vector4<f32>,
        out: &mut RenderLists,
    ) {
        let pos: na::Point3<f32> = na::convert(*pos);

        render::machine::render_cuboid_wireframe(
            &render::machine::Cuboid {
                // Slight z offset so that there is less overlap with e.g. the floor
                center: pos + na::Vector3::new(0.5, 0.5, 0.51),
                size: na::Vector3::new(1.0, 1.0, 1.0),
            },
            thickness,
            color,
            &mut out.plain,
        );
    }

    fn render_piece_to_place(
        &self,
        piece: &Piece,
        piece_pos: &grid::Point3,
        out: &mut RenderLists,
    ) {
        let mut any_pos_valid = false;

        for (pos, placed_block) in piece.iter_blocks(&piece_pos.coords) {
            let block_center = render::machine::block_center(&pos);
            let block_transform = render::machine::placed_block_transform(&placed_block);
            render::machine::render_block(
                &placed_block,
                0.0,
                &None,
                &block_center,
                &block_transform,
                0.8,
                out,
            );

            any_pos_valid = any_pos_valid || self.machine.is_valid_pos(&pos);

            if !self.machine.is_valid_pos(&pos) || self.machine.get_block_at_pos(&pos).is_some() {
                self.render_block_wireframe(
                    &pos,
                    0.025,
                    &na::Vector4::new(0.9, 0.0, 0.0, 1.0),
                    out,
                );
            }
        }

        // Show wireframe around whole piece only if there is at
        // least one block we can place at a valid position.
        if any_pos_valid {
            let piece_pos: na::Point3<f32> = na::convert(*piece_pos);
            let wire_size: na::Vector3<f32> = na::convert(piece.grid_size());
            let wire_center = piece_pos + wire_size / 2.0;
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

/// Actions that can be accessed by buttons and shortcuts in the editor.
impl Editor {
    pub fn action_undo(&mut self) {
        if let Some(undo_edit) = self.undo.pop_back() {
            let redo_edit = self.run_edit(undo_edit);
            self.redo.push(redo_edit);
        }
    }

    pub fn action_redo(&mut self) {
        if let Some(redo_edit) = self.redo.pop() {
            let undo_edit = self.run_edit(redo_edit);
            self.undo.push_back(undo_edit);
        }
    }

    pub fn action_cut(&mut self) {
        let edit = match &self.mode {
            Mode::Select(selection) => {
                self.clipboard = Some(Piece::new_from_selection(&self.machine, selection.iter()));

                // Note that `run_and_track_edit` will automatically clear the
                // selection, corresponding to the mutated machine.
                Some(Edit::SetBlocks(
                    selection.iter().map(|p| (*p, None)).collect(),
                ))
            }
            _ => {
                // No op in other modes.
                None
            }
        };

        if let Some(edit) = edit {
            self.run_and_track_edit(edit);
        }
    }

    pub fn action_copy(&mut self) {
        match &self.mode {
            Mode::Select(selection) => {
                self.clipboard = Some(Piece::new_from_selection(&self.machine, selection.iter()));
            }
            _ => {
                // No op in other modes.
            }
        }
    }

    pub fn action_paste(&mut self) {
        if let Some(clipboard) = &self.clipboard {
            // Kinda center the piece at the mouse
            self.mode = Mode::PlacePiece {
                piece: clipboard.clone(),
                offset: -clipboard.grid_center_xy(),
            };
        }
    }

    pub fn action_save(&mut self) {
        self.save(&self.config.default_save_path);
    }

    pub fn action_layer_up(&mut self) {
        if self.machine.is_valid_layer(self.current_layer + 1) {
            self.current_layer += 1;
        }
    }

    pub fn action_layer_down(&mut self) {
        if self.machine.is_valid_layer(self.current_layer - 1) {
            self.current_layer -= 1;
        }
    }

    pub fn action_select_mode(&mut self) {
        self.mode = Mode::Select(Vec::new());
    }

    pub fn action_rotate_cw(&mut self) {
        let mut edit = None;

        match &mut self.mode {
            Mode::PlacePiece { piece, offset } => {
                piece.rotate_cw_xy();
                *offset = -piece.grid_center_xy();
            }
            Mode::Select(selection) => {
                edit = Some(Edit::RotateCWXY(selection.clone()));
            }
            _ => {
                // No op in other modes.
            }
        };

        if let Some(edit) = edit {
            self.run_and_track_edit(edit);
        }
    }

    pub fn action_rotate_ccw(&mut self) {
        let mut edit = None;

        match &mut self.mode {
            Mode::PlacePiece { piece, offset } => {
                piece.rotate_ccw_xy();
                *offset = -piece.grid_center_xy();
            }
            Mode::Select(selection) => {
                edit = Some(Edit::RotateCCWXY(selection.clone()));
            }
            _ => {
                // No op in other modes.
            }
        };

        if let Some(edit) = edit {
            self.run_and_track_edit(edit);
        }
    }

    pub fn action_next_kind(&mut self) {
        match &mut self.mode {
            Mode::PlacePiece { piece, .. } => {
                piece.next_kind();
            }
            _ => {
                // No op in other modes.
            }
        };
    }
}
