use crate::grid::{Vec3, Grid3};

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
enum Axis3 {
    X,
    Y,
    Z,
}

impl Axis3 {
    pub fn to_vector(&self) -> Vec3 {
        match self {
            X => Vec3::new(1, 0, 0),
            Y => Vec3::new(0, 1, 0),
            Z => Vec3::new(0, 0, 1),
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
enum Sign {
    Pos,
    Neg,
}

impl Sign {
    pub fn to_number(&self) -> isize {
        match self {
            Pos => 1,
            Neg => -1,
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
struct Dir3(Axis3, Sign);

impl Dir3 {
    pub fn to_vector(&self, p: Vec3) -> Vec3 {
        self.0.to_vector() * self.1.to_number()
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
enum Block {
    Pipe {
        from: Dir3,
        to: Dir3,
    },
    Solid,
    Switch(Dir3),
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
struct BlockId(usize);

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
struct Blip {
    pos: Vec3,
}

#[derive(PartialEq, Eq, Clone, Debug)]
struct Machine {
    block_ids: Grid3<Option<BlockId>>,
    blocks: Vec<Block>,
    blips: Vec<Blip>,
}
