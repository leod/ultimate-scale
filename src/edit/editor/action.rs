use crate::edit::{Edit, Editor, Mode, Piece};
use crate::machine::grid;

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
            Mode::Select { selection, .. } => {
                self.clipboard = Some(Piece::new_from_selection(
                    &self.machine,
                    selection.iter().cloned(),
                ));

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
            Mode::Select { selection, .. } => {
                self.clipboard = Some(Piece::new_from_selection(
                    &self.machine,
                    selection.iter().cloned(),
                ));
            }
            _ => {
                // No op in other modes.
            }
        }
    }

    pub fn action_paste(&mut self) {
        if let Some(clipboard) = &self.clipboard {
            let mut piece = clipboard.clone();

            // Kinda center the piece at the mouse
            let mut extent = piece.extent();
            extent.z = 0;

            piece.shift(&(-piece.min_pos().coords - extent / 2));

            // Bias towards positive direction for even sizes.
            // Just feels more natural.
            // TODO: Bias actually needs to depend on the view position?
            if extent.x > 0 && extent.x % 2 == 0 {
                piece.shift(&grid::Vector3::x());
            }
            if extent.y > 0 && extent.y % 2 == 0 {
                piece.shift(&grid::Vector3::y());
            }

            self.mode = Mode::PlacePiece { piece };
        }
    }

    pub fn action_delete(&mut self) {
        let edit = match &self.mode {
            Mode::Select { selection, .. } => {
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
        self.mode = Mode::new_select();
    }

    pub fn action_pipe_tool_mode(&mut self) {
        self.mode = Mode::new_pipe_tool();
    }

    pub fn action_cancel(&mut self) {
        self.mode = match &self.mode {
            Mode::DragAndDrop { selection, .. } => Mode::new_selection(selection.clone()),
            Mode::PipeTool { last_pos, .. } if last_pos.is_some() => Mode::new_pipe_tool(),
            _ => Mode::new_select(),
        };
    }

    pub fn action_rotate_cw(&mut self) {
        let mut edit = None;

        match &mut self.mode {
            Mode::PlacePiece { piece } => {
                piece.rotate_cw_xy();
            }
            Mode::Select { selection, .. } => {
                if !selection.is_empty() {
                    edit = Some(Edit::RotateCWXY(selection.clone()));
                } else if let Some(mouse_block_pos) = self.mouse_block_pos {
                    edit = Some(Edit::RotateCWXY(vec![mouse_block_pos]));
                }
            }
            Mode::DragAndDrop { piece, .. } => {
                piece.rotate_cw_xy();
            }
            Mode::PipeTool { rotation_xy, .. } => {
                *rotation_xy += 1;
                if *rotation_xy == 4 {
                    *rotation_xy = 0;
                }
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
            Mode::PlacePiece { piece } => {
                piece.rotate_ccw_xy();
            }
            Mode::Select { selection, .. } => {
                if !selection.is_empty() {
                    edit = Some(Edit::RotateCCWXY(selection.clone()));
                } else if let Some(mouse_block_pos) = self.mouse_block_pos {
                    edit = Some(Edit::RotateCCWXY(vec![mouse_block_pos]));
                }
            }
            Mode::DragAndDrop { piece, .. } => {
                piece.rotate_ccw_xy();
            }
            Mode::PipeTool { rotation_xy, .. } => {
                if *rotation_xy == 0 {
                    *rotation_xy = 3;
                } else {
                    *rotation_xy -= 1;
                }
            }
            _ => {
                // No op in other modes.
            }
        };

        if let Some(edit) = edit {
            self.run_and_track_edit(edit);
        }
    }

    pub fn action_mirror_y(&mut self) {
        match &mut self.mode {
            Mode::PlacePiece { piece, .. } => {
                piece.mirror_y();
            }
            _ => {
                // No op in other modes.
            }
        }
    }

    pub fn action_next_kind(&mut self) {
        match &mut self.mode {
            Mode::PlacePiece { piece, .. } => {
                piece.set_next_kind();
            }
            _ => {
                // No op in other modes.
            }
        };
    }
}
