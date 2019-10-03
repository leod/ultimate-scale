pub mod config;
pub mod editor;

use crate::machine::grid;
use crate::machine::{Machine, PlacedBlock};

pub use config::Config;
pub use editor::Editor;

pub enum Edit {
    SetBlock(grid::Point3, Option<PlacedBlock>),
}

impl Edit {
    pub fn run(&self, machine: &mut Machine) -> Edit {
        match self {
            Edit::SetBlock(p, block) => {
                let previous_block = machine
                    .get_block_at_pos(p)
                    .map(|(_index, block)| block)
                    .cloned();
                machine.set_block_at_pos(p, block.clone());

                Edit::SetBlock(*p, previous_block)
            }
        }
    }
}
