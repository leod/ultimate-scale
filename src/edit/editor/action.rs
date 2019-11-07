use crate::edit::{Edit, Editor, Mode, Piece};

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
            // Kinda center the piece at the mouse
            self.mode = Mode::PlacePiece {
                piece: clipboard.clone(),
                offset: -clipboard.grid_center_xy(),
            };
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
            Mode::PlacePiece { piece, offset } => {
                piece.rotate_cw_xy();
                *offset = -piece.grid_center_xy();
            }
            Mode::Select { selection, .. } => {
                edit = Some(Edit::RotateCWXY(selection.clone()));
            }
            Mode::DragAndDrop { rotation_xy, .. } => {
                *rotation_xy += 1;
                if *rotation_xy == 4 {
                    *rotation_xy = 0;
                }
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
            Mode::PlacePiece { piece, offset } => {
                piece.rotate_ccw_xy();
                *offset = -piece.grid_center_xy();
            }
            Mode::Select { selection, .. } => {
                edit = Some(Edit::RotateCCWXY(selection.clone()));
            }
            Mode::DragAndDrop { rotation_xy, .. } => {
                if *rotation_xy == 0 {
                    *rotation_xy = 3;
                } else {
                    *rotation_xy -= 1;
                }
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
