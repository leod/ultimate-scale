pub mod config;
pub mod editor;
pub mod pick;
pub mod piece;

use std::collections::HashMap;

use nalgebra as na;

use crate::machine::grid;
use crate::machine::{Machine, PlacedBlock};

pub use config::Config;
pub use editor::Editor;
pub use piece::Piece;

#[derive(Debug, Clone)]
pub enum Edit {
    NoOp,
    SetBlocks(HashMap<grid::Point3, Option<PlacedBlock>>),

    /// Rotate blocks clockwise.
    RotateCWXY(Vec<grid::Point3>),

    /// Rotate blocks counterclockwise.
    RotateCCWXY(Vec<grid::Point3>),

    /// Run two edits in sequence.
    Pair(Box<Edit>, Box<Edit>),
}

impl Edit {
    /// Apply the edit operation to a machine and return an edit operation to
    /// undo what was done.
    pub fn run(self, machine: &mut Machine) -> Edit {
        match self {
            Edit::NoOp => Edit::NoOp,
            Edit::SetBlocks(blocks) => {
                let valid_blocks = blocks
                    .into_iter()
                    .filter(|(p, _block)| machine.is_valid_pos(p))
                    .collect::<HashMap<_, _>>();

                let previous_blocks = valid_blocks
                    .keys()
                    .map(|p| {
                        (
                            *p,
                            machine
                                .get_block_at_pos(p)
                                .map(|(_index, block)| block.clone()),
                        )
                    })
                    .collect();

                if previous_blocks == valid_blocks {
                    Edit::NoOp
                } else {
                    for (p, block) in valid_blocks.iter() {
                        machine.set_block_at_pos(p, block.clone());
                    }

                    Edit::SetBlocks(previous_blocks)
                }
            }
            Edit::RotateCWXY(points) => {
                for p in &points {
                    if let Some((_, placed_block)) = machine.get_block_at_pos_mut(p) {
                        placed_block.rotate_cw_xy();
                    }
                }

                if points.is_empty() {
                    Edit::NoOp
                } else {
                    Edit::RotateCCWXY(points)
                }
            }
            Edit::RotateCCWXY(points) => {
                for p in &points {
                    if let Some((_, placed_block)) = machine.get_block_at_pos_mut(p) {
                        placed_block.rotate_ccw_xy();
                    }
                }

                if points.is_empty() {
                    Edit::NoOp
                } else {
                    Edit::RotateCWXY(points)
                }
            }
            Edit::Pair(a, b) => {
                let undo_a = a.run(machine);
                let undo_b = b.run(machine);

                Self::compose(undo_b, undo_a)
            }
        }
    }

    pub fn compose(a: Edit, b: Edit) -> Edit {
        match (a, b) {
            (Edit::NoOp, Edit::NoOp) => Edit::NoOp,
            (Edit::SetBlocks(mut a), Edit::SetBlocks(b)) => {
                for (p, block) in b.into_iter() {
                    a.insert(p, block);
                }

                Edit::SetBlocks(a)
            }
            (a, b) => Edit::Pair(Box::new(a), Box::new(b)),
        }
    }
}

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
