pub mod editor;

use crate::machine::grid;
use crate::machine::{Block, Machine};

pub use editor::Editor;

pub enum Edit {
    SetBlock(grid::Point3, Option<Block>),
}

impl Edit {
    pub fn run(&self, machine: &mut Machine) -> Edit { 
        match self {
            Edit::SetBlock(p, block) => {
                let previous_block = machine.get_block(p).cloned();
                machine.set_block(p, block.clone());

                Edit::SetBlock(*p, previous_block)
            }
        }
    }
}
