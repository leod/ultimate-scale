pub mod grid;
pub mod exec;

use crate::util::vec_option::VecOption;

use grid::{Vector3, Point3, Grid3};

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum Axis3 {
    X,
    Y,
    Z,
}

impl Axis3 {
    pub fn to_vector(&self) -> Vector3 {
        match self {
            Axis3::X => Vector3::x(),
            Axis3::Y => Vector3::y(),
            Axis3::Z => Vector3::z(),
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum Sign {
    Pos,
    Neg,
}

impl Sign {
    pub fn to_number(&self) -> isize {
        match self {
            Sign::Pos => 1,
            Sign::Neg => -1,
        }
    }

    pub fn invert(&self) -> Sign {
        match self {
            Sign::Pos => Sign::Neg,
            Sign::Neg => Sign::Pos,
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct Dir3(Axis3, Sign);

impl Dir3 {
    pub fn to_vector(&self) -> Vector3 {
        self.0.to_vector() * self.1.to_number()
    }

    pub fn invert(&self) -> Dir3 {
        Dir3(self.0, self.1.invert())
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum Block {
    Pipe {
        from: Dir3,
        to: Dir3,
    },
    Switch(Dir3),
    Solid,
}

impl Block {
}

pub type BlockId = usize;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Blocks {
    pub ids: Grid3<Option<BlockId>>,
    pub data: VecOption<Block>,
}

impl Blocks {
    pub fn new(size: Vector3) -> Blocks {
        Blocks {
            ids: Grid3::new(size),
            data: VecOption::new(),
        }
    }

    pub fn at_pos(&self, p: &Point3) -> Option<&Block> {
        self
            .ids
            .get(p)
            .and_then(|id| id.as_ref())
            .map(|&id| &self.data[id])
    }

    pub fn ids(&self) -> &Grid3<Option<BlockId>> {
        &self.ids
    }

    pub fn is_valid_pos(&self, p: &Point3) -> bool {
        self.ids.is_valid_pos(p)
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

    pub fn blocks(&self) -> &Blocks {
        &self.blocks
    }
}
