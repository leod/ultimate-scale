pub mod config;
pub mod editor;
pub mod pick;

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
    blocks: HashMap<grid::Point3, PlacedBlock>,
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
        Piece {
            blocks: Self::blocks_to_origin(blocks),
        }
    }

    pub fn grid_size(&self) -> grid::Vector3 {
        let mut max = grid::Vector3::zeros();

        for p in self.blocks.keys() {
            if p.x > max.x {
                max.x = p.x;
            }
            if p.y > max.y {
                max.y = p.y;
            }
            if p.z > max.z {
                max.z = p.z;
            }
        }

        max + grid::Vector3::new(1, 1, 1)
    }

    pub fn rotate_cw_xy(&mut self) {
        self.blocks = Self::blocks_to_origin(
            self.blocks
                .clone()
                .into_iter()
                .map(|(p, mut placed_block)| {
                    let rotated_p = grid::Point3::new(p.y, self.grid_size().y - p.x, p.z);

                    placed_block.rotate_cw_xy();

                    (rotated_p, placed_block)
                })
                .collect(),
        );
    }

    pub fn rotate_ccw_xy(&mut self) {
        for _ in 0..3 {
            self.rotate_cw_xy();
        }
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

    pub fn blocks_to_origin(
        blocks: HashMap<grid::Point3, PlacedBlock>,
    ) -> HashMap<grid::Point3, PlacedBlock> {
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

        blocks
            .into_iter()
            .map(|(p, block)| (p - min, block))
            .collect()
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
