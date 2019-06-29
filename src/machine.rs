use crate::grid::Grid3;

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

struct Machine {
    block_ids: Grid3<Option<BlockId>>,
    blocks: Vec<Block>,
}
