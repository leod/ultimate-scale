pub mod config;
pub mod editor;

use std::collections::{HashMap, HashSet};

use crate::machine::grid;
use crate::machine::{Machine, PlacedBlock};

pub use config::Config;
pub use editor::Editor;

pub enum Edit {
    SetBlocks(HashMap<grid::Point3, Option<PlacedBlock>>),
}

impl Edit {
    pub fn run(&self, machine: &mut Machine) -> Edit {
        match self {
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

                for (p, block) in blocks.iter() {
                    machine.set_block_at_pos(p, block.clone());
                }

                Edit::SetBlocks(previous_blocks)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Select(HashSet<grid::Point3>),
    PlaceBlock(PlacedBlock),
}
