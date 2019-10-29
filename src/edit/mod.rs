pub mod config;
pub mod editor;

use std::collections::{HashMap, HashSet};

use crate::machine::grid;
use crate::machine::{Machine, PlacedBlock};

pub use config::Config;
pub use editor::Editor;

/// A piece of a machine that can be kept around as edit actions, or in the
/// clipboard and stuff like that.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Piece {
    /// Blocks that shall be placed. All point coordinates are assumed to be
    /// non-negative.
    pub blocks: HashMap<grid::Point3, PlacedBlock>,
}

impl Piece {
    pub fn new_origin_block(block: PlacedBlock) -> Self {
        Self {
            blocks: maplit::hashmap! {
                grid::Point3::origin() => block,
            },
        }
    }

    pub fn new_blocks_to_origin(blocks: HashMap<grid::Point3, PlacedBlock>) -> Piece {
        let mut min = grid::Vector3::new(std::isize::MAX, std::isize::MAX, std::isize::MAX);

        for p in blocks.keys() {
            if p.x < min.x {
                min.x = p.x;
            }
            if p.y < min.y {
                min.y = p.y;
            }
            if p.z < min.z {
                min.z = p.z;
            }
        }

        let blocks_at_origin = blocks
            .into_iter()
            .map(|(p, block)| (p - min, block))
            .collect();

        Piece {
            blocks: blocks_at_origin,
        }
    }

    pub fn rotate_cw(&mut self) {
        // TODO
    }

    pub fn rotate_ccw(&mut self) {
        // TODO
    }

    pub fn place_edit(&self, offset: &grid::Vector3) -> Edit {
        let set_blocks = self
            .iter_blocks(offset)
            .map(|(pos, block)| (pos, Some(block)))
            .collect();

        Edit::SetBlocks(set_blocks)
    }

    pub fn iter_blocks(
        &self,
        offset: &grid::Vector3,
    ) -> impl Iterator<Item = (grid::Point3, PlacedBlock)> + '_ {
        let offset = *offset;
        self.blocks
            .iter()
            .map(move |(pos, block)| (pos + offset, block.clone()))
    }
}

#[derive(Debug, Clone)]
pub enum Edit {
    NoOp,
    SetBlocks(HashMap<grid::Point3, Option<PlacedBlock>>),
}

impl Edit {
    /// Apply the edit operation to a machine and return an edit operation to
    /// undo what was done.
    pub fn run(&self, machine: &mut Machine) -> Edit {
        match self {
            Edit::NoOp => Edit::NoOp,
            Edit::SetBlocks(blocks) => {
                let previous_blocks = blocks
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

                if *blocks == previous_blocks {
                    Edit::NoOp
                } else {
                    for (p, block) in blocks.iter() {
                        machine.set_block_at_pos(p, block.clone());
                    }

                    Edit::SetBlocks(previous_blocks)
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Select(HashSet<grid::Point3>),
    PlacePiece(Piece),
}
