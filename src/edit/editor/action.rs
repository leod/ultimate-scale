use crate::edit::{Edit, Editor, Mode, Piece};
use crate::machine::{grid, Block, PlacedBlock};

#[allow(unused)]
/// Actions that can be accessed by buttons and shortcuts in the editor.
/// This has now been turned into an enum to allow UI to run in the main
/// thread and send back actions to the update thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Delete,
    Save,
    LayerUp,
    LayerDown,
    SelectAll,
    SelectMode,
    SelectLayerBoundMode,
    PipeToolMode,
    PlaceBlockMode(Block),
    Cancel,
    RotateCW,
    RotateCCW,
    MirrorY,
    NextKind,
}

impl Editor {
    pub fn run_action(&mut self, action: Action) {
        match action {
            Action::Undo => self.action_undo(),
            Action::Redo => self.action_redo(),
            Action::Cut => self.action_cut(),
            Action::Copy => self.action_copy(),
            Action::Paste => self.action_paste(),
            Action::Delete => self.action_delete(),
            Action::Save => self.action_save(),
            Action::LayerUp => self.action_layer_up(),
            Action::LayerDown => self.action_layer_down(),
            Action::SelectAll => self.action_select_all(),
            Action::SelectMode => self.action_select_mode(),
            Action::SelectLayerBoundMode => self.action_select_layer_bound_mode(),
            Action::PipeToolMode => self.action_pipe_tool_mode(),
            Action::PlaceBlockMode(block) => self.action_place_block_mode(block),
            Action::Cancel => self.action_cancel(),
            Action::RotateCW => self.action_rotate_cw(),
            Action::RotateCCW => self.action_rotate_ccw(),
            Action::MirrorY => self.action_mirror_y(),
            Action::NextKind => self.action_next_kind(),
        }
    }

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
        if let Some(selection) = self.mode.selection() {
            self.clipboard = Some(Piece::new_from_selection(
                &self.machine,
                selection.iter().cloned(),
            ));
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

            // If we are placing in an upper layer, it could be that the piece
            // sticks out at the top. Shift down if that is the case.
            let max_z = piece.blocks().iter().map(|(p, _)| p.z).max().unwrap_or(0)
                + self.mouse_grid_pos.map_or(0, |p| p.z);
            let too_high = (max_z - self.machine().size().z + 1).max(0);

            self.current_layer -= too_high.min(self.current_layer);
            assert!(self.machine.is_valid_layer(self.current_layer));

            self.mode = self.mode.clone().switch_to_place_piece(piece, true);
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
        } else {
            let piece = match &mut self.mode {
                Mode::DragAndDrop { piece, .. } => Some(piece),
                Mode::PlacePiece { piece, .. } => Some(piece),
                _ => None,
            };

            if let Some(piece) = piece {
                // Similar to `action_layer_down`.
                if self.current_layer + piece.min_pos().z + 1 < self.machine.size().z {
                    piece.shift(&grid::Vector3::z());
                }
            }
        }
    }

    pub fn action_layer_down(&mut self) {
        if self.machine.is_valid_layer(self.current_layer - 1) {
            self.current_layer -= 1;
        } else {
            let piece = match &mut self.mode {
                Mode::DragAndDrop { piece, .. } => Some(piece),
                Mode::PlacePiece { piece, .. } => Some(piece),
                _ => None,
            };

            if let Some(piece) = piece {
                // Here we may have the case that we are dragging a piece in
                // layer e.g. 3, while the editor is set to layer 0. Then the
                // user cannot drag the object to any layer below 3, because
                // we disallow setting the editor to layers below 0. Thus, we
                // shift the piece down instead.
                if self.current_layer + piece.max_pos().z > 0 {
                    piece.shift(&-grid::Vector3::z());
                }
            }
        }
    }

    pub fn action_select_all(&mut self) {
        self.mode = self.overwrite_selection(
            self.machine.iter_blocks().map(|(_, (pos, _))| *pos),
            self.mode.clone(),
        );
    }

    pub fn action_select_mode(&mut self) {
        self.go_into_select_mode(false);
    }

    pub fn action_select_layer_bound_mode(&mut self) {
        self.go_into_select_mode(true);
    }

    pub fn action_pipe_tool_mode(&mut self) {
        self.mode = Mode::new_pipe_tool();
    }

    pub fn action_place_block_mode(&mut self, block: Block) {
        // TODO: Maintain current rotation when switching to a different block
        // to place.
        let piece = Piece::new_origin_block(PlacedBlock { block });

        self.mode = self.mode.clone().switch_to_place_piece(piece, false);
    }

    pub fn action_cancel(&mut self) {
        self.mode = match &self.mode {
            Mode::DragAndDrop { selection, .. } => Mode::new_selection(selection.clone()),
            Mode::PipeTool { last_pos, .. } if last_pos.is_some() => Mode::new_pipe_tool(),
            Mode::PlacePiece { outer, .. } => (**outer).clone(),
            _ => Mode::new_select(),
        };
    }

    pub fn action_rotate_cw(&mut self) {
        let mut edit = None;

        match &mut self.mode {
            Mode::PlacePiece { piece, .. } => {
                piece.rotate_cw_xy();
            }
            Mode::Select { .. } => {
                if let Some(mouse_block_pos) = self.mouse_block_pos {
                    edit = Some(Edit::RotateCWXY(vec![mouse_block_pos]));
                }
            }
            Mode::DragAndDrop { piece, .. } => {
                piece.rotate_cw_xy();
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
            Mode::PlacePiece { piece, .. } => {
                piece.rotate_ccw_xy();
            }
            Mode::Select { .. } => {
                if let Some(mouse_block_pos) = self.mouse_block_pos {
                    edit = Some(Edit::RotateCCWXY(vec![mouse_block_pos]));
                }
            }
            Mode::DragAndDrop { piece, .. } => {
                piece.rotate_ccw_xy();
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
        let mut edit = None;

        match &mut self.mode {
            Mode::PlacePiece { piece, .. } => {
                piece.set_next_kind();
            }
            Mode::Select { selection, .. } => {
                if !selection.is_empty() {
                    edit = Some(Edit::NextKind(selection.to_vec()));
                } else if let Some(mouse_block_pos) = self.mouse_block_pos {
                    edit = Some(Edit::NextKind(vec![mouse_block_pos]));
                }
            }
            Mode::DragAndDrop { piece, .. } => {
                piece.set_next_kind();
            }
            _ => {
                // No op in other modes.
            }
        };

        if let Some(edit) = edit {
            self.run_and_track_edit(edit);
        }
    }
}
