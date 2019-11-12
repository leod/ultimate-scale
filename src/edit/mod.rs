pub mod config;
pub mod editor;
pub mod mode;
pub mod pick;
pub mod piece;

use std::collections::HashMap;

use crate::machine::grid;
use crate::machine::{Machine, PlacedBlock};

pub use config::Config;
pub use editor::Editor;
pub use mode::Mode;
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
                        placed_block.block.mutate_dirs(|dir| dir.rotated_cw_xy());
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
                        placed_block.block.mutate_dirs(|dir| dir.rotated_ccw_xy());
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
            (Edit::NoOp, b) => b,
            (a, Edit::NoOp) => a,
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
