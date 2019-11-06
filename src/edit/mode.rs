use nalgebra as na;

use crate::edit::Piece;
use crate::machine::{grid, Machine};

/// Modes that the editor can be in.
#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    /// Select blocks in the machine.
    ///
    /// For consistency, the selected positions must always contain a block.
    /// There must be no duplicate positions. The order corresponds to the
    /// selection order.
    Select(Vec<grid::Point3>),

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
        offset: grid::Vector3,
    },

    DragAndDrop {
        /// Selection that is being dragged. No duplicate positions, and each
        /// must contain a block in the machine.
        selection: Vec<grid::Point3>,

        /// Position that is being dragged, i.e. the block that was grabbed by
        /// the user.
        center_pos: grid::Point3,

        /// Rotation to be applied to the piece.
        rotation_xy: usize,

        layer_offset: isize,
    },

    PipeTool {
        last_pos: Option<grid::Point3>,
        rotation_xy: usize,
    },
}

impl Mode {
    /// Make sure the mode state is consistent with the edited machine.
    ///
    /// The main case is for this to be called after an edit has been applied to
    /// the machine. In that case, the edit may have cleared out a block
    /// position which is currently selected in a `Mode`, so we need to remove
    /// it from the selection.
    pub fn make_consistent_with_machine(self, machine: &Machine) -> Self {
        match self {
            Mode::Select(mut selection) => {
                selection.retain(|grid_pos| machine.get_block_at_pos(grid_pos).is_some());
                Mode::Select(selection)
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
                center_pos,
                rotation_xy,
                layer_offset,
            } => {
                selection.retain(|grid_pos| machine.get_block_at_pos(grid_pos).is_some());

                if !selection.contains(&center_pos) {
                    // If the center block is not selected anymore, let's just
                    // not bother with this.
                    Mode::Select(selection)
                } else {
                    Mode::DragAndDrop {
                        selection,
                        center_pos,
                        rotation_xy,
                        layer_offset,
                    }
                }
            }
            mode => mode,
        }
    }
}
