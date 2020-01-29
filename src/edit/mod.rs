pub mod config;
pub mod editor;
pub mod mode;
pub mod pick;
pub mod piece;

use std::collections::HashMap;

use crate::machine::grid;
use crate::machine::{Block, Machine, PlacedBlock};

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

    /// Switch to the next kind.
    NextKind(Vec<grid::Point3>),

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

                let mut previous_blocks: HashMap<_, _> = valid_blocks
                    .keys()
                    .map(|p| (*p, machine.get(p).cloned()))
                    .collect();

                // Make sure that we conserve input and output blocks.
                let counts_before = (
                    count_inputs(previous_blocks.values()),
                    count_outputs(previous_blocks.values()),
                );
                let counts_after = (
                    count_inputs(valid_blocks.values()),
                    count_outputs(valid_blocks.values()),
                );

                if previous_blocks == valid_blocks || counts_before != counts_after {
                    Edit::NoOp
                } else {
                    for (p, block) in valid_blocks.iter() {
                        machine.set(p, block.clone());

                        // We may wish to update the connectivity of
                        // neighboring blocks (specifically `GeneralPipe`).
                        for &dir in &grid::Dir3::ALL {
                            let neighbor_p = p + dir.to_vector();

                            if valid_blocks.contains_key(&neighbor_p) {
                                // The block at this neighbor's position is
                                // being overwritten anyway, so we can ignore
                                // it here.
                                continue;
                            }

                            if block
                                .as_ref()
                                .map_or(false, |block| block.block.has_wind_hole(dir))
                            {
                                // No need to change the neighbor's connectivity.
                                continue;
                            }

                            if let Some(neighbor_block) = machine.get_mut(&(p + dir.to_vector())) {
                                let previous_block = neighbor_block.clone();

                                if let Block::GeneralPipe(dirs) = &mut neighbor_block.block {
                                    // Cut off this direction from the
                                    // neighboring `GeneralPipe`.
                                    if dirs[dir.invert()] {
                                        dirs[dir.invert()] = false;
                                    }

                                    // And remember how to undo this.
                                    assert!(!previous_blocks.contains_key(&neighbor_p));
                                    previous_blocks.insert(neighbor_p, Some(previous_block));
                                }
                            }
                        }
                    }

                    Edit::SetBlocks(previous_blocks)
                }
            }
            Edit::RotateCWXY(points) => {
                for p in &points {
                    if let Some(placed_block) = machine.get_mut(p) {
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
                    if let Some(placed_block) = machine.get_mut(p) {
                        placed_block.block.mutate_dirs(|dir| dir.rotated_ccw_xy());
                    }
                }

                if points.is_empty() {
                    Edit::NoOp
                } else {
                    Edit::RotateCWXY(points)
                }
            }
            Edit::NextKind(points) => {
                for p in &points {
                    if let Some(placed_block) = machine.get_mut(p) {
                        if let Some(kind) = placed_block.block.kind() {
                            placed_block.block.set_kind(kind.next());
                        }
                    }
                }

                if points.is_empty() {
                    Edit::NoOp
                } else {
                    // TODO: Undo for `Edit::NextKinds` needs to change if we
                    // ever add more blip kinds.
                    Edit::NextKind(points)
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

pub fn count_inputs<'a>(blocks: impl Iterator<Item = &'a Option<PlacedBlock>>) -> usize {
    blocks
        .map(|block| match block {
            Some(PlacedBlock {
                block: Block::Input { .. },
            }) => 1,
            _ => 0,
        })
        .sum()
}

pub fn count_outputs<'a>(blocks: impl Iterator<Item = &'a Option<PlacedBlock>>) -> usize {
    blocks
        .map(|block| match block {
            Some(PlacedBlock {
                block: Block::Output { .. },
            }) => 1,
            _ => 0,
        })
        .sum()
}
