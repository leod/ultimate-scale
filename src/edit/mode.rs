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
    Select {
        selection: Vec<grid::Point3>,
    },

    /// User just clicked on a block in selection mode.
    ///
    /// Based on this, we will switch to `DragAndDrop` if the mouse grid
    /// position changes.
    SelectClickedOnBlock {
        selection: Vec<grid::Point3>,

        /// The position of the block the user clicked on.
        dragged_block_pos: grid::Point3,

        /// The mouse grid position at the time of the click.
        dragged_grid_pos: grid::Point3,
    },

    /// Select blocks in the machine by a screen rectangle.
    RectSelect {
        /// Blocks that were already selected when entering this mode.
        existing_selection: Vec<grid::Point3>,

        /// New blocks currently selected by the rectangle.
        new_selection: Vec<grid::Point3>,

        /// Start position of the rectangle.
        start_pos: na::Point2<f32>,

        /// Current end position of the rectangle.
        end_pos: na::Point2<f32>,
    },

    PlacePiece {
        piece: Piece,
    },

    DragAndDrop {
        /// Blocks that were selected at the time of starting drag-and-drop.
        /// Used for returning to selection mode if drag-and-drop is aborted.
        selection: Vec<grid::Point3>,

        piece: Piece,
    },

    PipeTool {
        last_pos: Option<grid::Point3>,
        rotation_xy: usize,
        blocks: HashMap<grid::Point3, PlacedBlock>,
    },
}

impl Mode {
    pub fn new_select() -> Self {
        Self::new_selection(Vec::new())
    }

    pub fn new_selection(selection: Vec<grid::Point3>) -> Self {
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

    /// Make sure the mode state is consistent with the edited machine.
    ///
    /// The main case is for this to be called after an edit has been applied to
    /// the machine. In that case, the edit may have cleared out a block
    /// position which is currently selected in a `Mode`, so we need to remove
    /// it from the selection.
    pub fn make_consistent_with_machine(self, machine: &Machine) -> Self {
        match self {
            Mode::Select { mut selection } => {
                selection.retain(|grid_pos| machine.get_block_at_pos(grid_pos).is_some());

                Mode::Select { selection }
            }
            Mode::SelectClickedOnBlock {
                mut selection,
                dragged_block_pos,
                dragged_grid_pos,
            } => {
                selection.retain(|grid_pos| machine.get_block_at_pos(grid_pos).is_some());

                if machine.get_block_at_pos(&dragged_block_pos).is_none() {
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
                mut existing_selection,
                mut new_selection,
                start_pos,
                end_pos,
            } => {
                existing_selection.retain(|grid_pos| machine.get_block_at_pos(grid_pos).is_some());
                new_selection.retain(|grid_pos| machine.get_block_at_pos(grid_pos).is_some());

                Mode::RectSelect {
                    existing_selection,
                    new_selection,
                    start_pos,
                    end_pos,
                }
            }
            Mode::DragAndDrop {
                mut selection,
                piece,
            } => {
                selection.retain(|grid_pos| machine.get_block_at_pos(grid_pos).is_some());

                Mode::DragAndDrop { selection, piece }
            }
            mode => mode,
        }
    }
}
