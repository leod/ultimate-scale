use std::collections::HashMap;

use nalgebra as na;

use crate::edit::Piece;
use crate::machine::{grid, Machine, PlacedBlock};

/// Modes that the editor can be in.
#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    /// Select blocks in the machine.
    ///
    /// For consistency, the selected positions must always contain a block.
    /// There must be no duplicate positions. The order corresponds to the
    /// selection order.
    Select { selection: SelectionMode },

    /// User just clicked on a block in selection mode.
    ///
    /// Based on this, we will switch to `DragAndDrop` if the mouse grid
    /// position changes.
    SelectClickedOnBlock {
        selection: SelectionMode,

        /// The position of the block the user clicked on.
        dragged_block_pos: grid::Point3,

        /// The mouse grid position at the time of the click.
        dragged_grid_pos: grid::Point3,
    },

    DragAndDrop {
        /// Blocks that were selected at the time of starting drag-and-drop.
        /// Used for returning to selection mode if drag-and-drop is aborted.
        selection: SelectionMode,

        piece: Piece,
    },

    /// Select blocks in the machine by a screen rectangle.
    RectSelect {
        /// Blocks that were already selected when entering this mode.
        existing_selection: SelectionMode,

        /// New blocks currently selected by the rectangle.
        new_selection: Vec<grid::Point3>,

        /// Start position of the rectangle.
        start_pos: na::Point2<f32>,

        /// Current end position of the rectangle.
        end_pos: na::Point2<f32>,
    },

    PlacePiece {
        piece: Piece,
        is_paste: bool,
        outer: Box<Mode>,
    },

    PipeTool {
        last_pos: Option<grid::Point3>,
        rotation_xy: usize,
        blocks: HashMap<grid::Point3, PlacedBlock>,
    },
}

impl Mode {
    pub fn new_select() -> Self {
        Self::new_selection(SelectionMode::new(false))
    }

    pub fn new_selection(selection: SelectionMode) -> Self {
        Mode::Select { selection }
    }

    pub fn new_pipe_tool() -> Self {
        Self::new_pipe_tool_with_rotation(1)
    }

    pub fn new_pipe_tool_with_rotation(rotation_xy: usize) -> Self {
        Mode::PipeTool {
            last_pos: None,
            rotation_xy,
            blocks: HashMap::new(),
        }
    }

    pub fn switch_to_place_piece(self, piece: Piece, is_paste: bool) -> Self {
        match self {
            Mode::PlacePiece { outer, .. } => Mode::PlacePiece {
                piece,
                is_paste,
                outer,
            },
            x => Mode::PlacePiece {
                piece,
                is_paste,
                outer: Box::new(x),
            },
        }
    }

    pub fn selection(&self) -> Option<&SelectionMode> {
        match self {
            Mode::Select { selection } => Some(selection),
            Mode::SelectClickedOnBlock { selection, .. } => Some(selection),
            Mode::DragAndDrop { selection, .. } => Some(selection),
            Mode::RectSelect {
                existing_selection, ..
            } => Some(existing_selection),
            _ => None,
        }
    }

    /// Make sure the mode state is consistent with the edited machine.
    ///
    /// The main case is for this to be called after an edit has been applied to
    /// the machine. In that case, the edit may have cleared out a block
    /// position which is currently selected in a `Mode`, so we need to remove
    /// it from the selection.
    pub fn make_consistent_with_machine(self, machine: &Machine) -> Self {
        match self {
            Mode::Select { selection } => {
                let selection = selection.make_consistent_with_machine(machine);

                Mode::Select { selection }
            }
            Mode::SelectClickedOnBlock {
                selection,
                dragged_block_pos,
                dragged_grid_pos,
            } => {
                let selection = selection.make_consistent_with_machine(machine);

                if !machine.is_block_at(&dragged_block_pos) {
                    // The block that the user want to drag-and-drop was
                    // removed; give up on dragging.
                    Mode::Select { selection }
                } else {
                    // Keep on trying.
                    Mode::SelectClickedOnBlock {
                        selection,
                        dragged_block_pos,
                        dragged_grid_pos,
                    }
                }
            }
            Mode::RectSelect {
                existing_selection,
                mut new_selection,
                start_pos,
                end_pos,
            } => {
                let existing_selection = existing_selection.make_consistent_with_machine(machine);
                new_selection.retain(|grid_pos| machine.is_block_at(grid_pos));

                Mode::RectSelect {
                    existing_selection,
                    new_selection,
                    start_pos,
                    end_pos,
                }
            }
            Mode::DragAndDrop { selection, piece } => {
                let selection = selection.make_consistent_with_machine(machine);

                Mode::DragAndDrop { selection, piece }
            }
            Mode::PlacePiece {
                piece,
                is_paste,
                outer,
            } => {
                let new_outer = (*outer).clone().make_consistent_with_machine(machine);

                Mode::PlacePiece {
                    piece,
                    is_paste,
                    outer: Box::new(new_outer),
                }
            }
            mode => mode,
        }
    }

    pub fn is_layer_bound(&self) -> bool {
        match self {
            Mode::Select { selection, .. } => selection.is_layer_bound(),
            Mode::SelectClickedOnBlock { selection, .. } => selection.is_layer_bound(),
            Mode::DragAndDrop { selection, .. } => selection.is_layer_bound(),
            Mode::RectSelect {
                existing_selection, ..
            } => existing_selection.is_layer_bound(),
            Mode::PlacePiece { .. } => true,
            Mode::PipeTool { .. } => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectionMode {
    points: Vec<grid::Point3>,
    is_layer_bound: bool,
}

impl SelectionMode {
    pub fn new(is_layer_bound: bool) -> SelectionMode {
        SelectionMode {
            points: Vec::new(),
            is_layer_bound,
        }
    }

    fn push(&mut self, p: grid::Point3) {
        // Make sure that the point unique and is at the end of the vector.
        self.points.retain(|q| *q != p);
        self.points.push(p);
    }

    pub fn push_if_correct_layer(&mut self, current_layer: isize, p: grid::Point3) {
        if !self.is_layer_bound || current_layer == p.z {
            self.push(p);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &grid::Point3> {
        self.points.iter()
    }

    pub fn is_layer_bound(&self) -> bool {
        self.is_layer_bound
    }

    pub fn contains(&self, p: &grid::Point3) -> bool {
        self.points.contains(p)
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn clear(&mut self) {
        self.points.clear();
    }

    pub fn newest_point(&self) -> Option<grid::Point3> {
        self.points.last().copied()
    }

    pub fn to_vec(&self) -> Vec<grid::Point3> {
        self.points.clone()
    }

    pub fn toggle(&mut self, p: &grid::Point3) {
        if self.points.contains(p) {
            self.points.retain(|q| q != p);
        } else {
            self.points.push(*p);
        }
    }

    pub fn make_consistent_with_machine(mut self, machine: &Machine) -> Self {
        self.points.retain(|p| machine.is_block_at(p));
        self
    }

    pub fn set_is_layer_bound(&mut self, current_layer: isize, new_is_layer_bound: bool) {
        if !self.is_layer_bound && new_is_layer_bound {
            self.points.retain(|q| q.z == current_layer);
        }

        self.is_layer_bound = new_is_layer_bound;
    }
}
