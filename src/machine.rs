use crate::grid::{Pos3, Grid3};

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
enum Axis {
    X,
    Y,
    Z,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
enum Sign {
    Pos,
    Neg,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
struct Dir(Axis, Sign);

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
enum Block {
    Pipe(Dir),
    Solid,
    Switch(Dir),
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
struct BlockId(usize);

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
struct Blip {
    pos: Pos3,
}

#[derive(PartialEq, Eq, Clone, Debug)]
struct Machine {
    block_ids: Grid3<Option<BlockId>>,
    blocks: Vec<Block>,
    blips: Vec<Blip>,
}
