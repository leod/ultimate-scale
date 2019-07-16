pub mod grid;
pub mod exec;

use crate::util::vec_option::{self, VecOption};

use grid::{Vector3, Point3, Dir3, Grid3};

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum Block {
    PipeXY,
    PipeSplitXY,
    PipeBendXY,
    Solid,
}

impl Block {
}

#[derive(PartialEq, Eq, Clone, Debug, Default)]
pub struct WindState {
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct PlacedBlock {
    pub dir_xy: grid::Dir2,
    pub block: Block,
    pub wind_state: WindState,
}

pub type BlockId = usize;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Blocks {
    pub ids: Grid3<Option<BlockId>>,
    pub data: VecOption<(Point3, PlacedBlock)>,
}

impl Blocks {
    pub fn new(size: Vector3) -> Blocks {
        Blocks {
            ids: Grid3::new(size),
            data: VecOption::new(),
        }
    }

    pub fn get(&self, p: &Point3) -> Option<&PlacedBlock> {
        self
            .ids
            .get(p)
            .and_then(|id| id.as_ref())
            .map(|&id| &self.data[id].1)
    }

    pub fn remove(&mut self, p: &Point3) -> Option<PlacedBlock> {
        if let Some(Some(id)) = self.ids.get(p).cloned() {
            self.ids[*p] = None;
            self.data.remove(id).map(|(id, block)| block)
        } else {
            None
        }
    }

    pub fn set(&mut self, p: &Point3, block: Option<PlacedBlock>) {
        self.remove(p);

        if let Some(block) = block {
            let id = self.data.add((*p, block));
            self.ids[*p] = Some(id);
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Machine {
    pub(in crate::machine) blocks: Blocks,
}

impl Machine {
    pub fn new(size: Vector3) -> Machine {
        Machine {
            blocks: Blocks::new(size),
        }
    }

    pub fn size(&self) -> Vector3 {
        self.blocks.ids.size()
    }

    pub fn is_valid_pos(&self, p: &Point3) -> bool {
        self.blocks.ids.is_valid_pos(p)
    }

    pub fn is_valid_layer(&self, layer: isize) -> bool {
        layer >= 0 && layer < self.size().z
    }

    pub fn get_block(&self, p: &Point3) -> Option<&PlacedBlock> {
        self.blocks.get(p)
    }

    pub fn set_block(&mut self, p: &Point3, block: Option<PlacedBlock>) {
        self.blocks.set(p, block);
    }

    pub fn iter_blocks(&self) -> impl Iterator<Item=(Point3, &PlacedBlock)> {
        self
            .blocks
            .data
            .iter()
            .map(|(_, &(pos, ref block))| (pos, block))
    }
}
