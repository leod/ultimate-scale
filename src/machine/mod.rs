mod grid;

use crate::vec_option::VecOption;

use grid::{Vec3, Grid3};

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
enum Axis3 {
    X,
    Y,
    Z,
}

impl Axis3 {
    pub fn to_vector(&self) -> Vec3 {
        match self {
            Axis3::X => Vec3::new(1, 0, 0),
            Axis3::Y => Vec3::new(0, 1, 0),
            Axis3::Z => Vec3::new(0, 0, 1),
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
struct Dir3(Axis3, Sign);

impl Dir3 {
    pub fn to_vector(&self) -> Vec3 {
        self.0.to_vector() * self.1.to_number()
    }

    pub fn invert(&self) -> Dir3 {
        Dir3(self.0, self.1.invert())
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
enum Block {
    Pipe {
        from: Dir3,
        to: Dir3,
    },
    Switch(Dir3),
    Solid,
}

impl Block {
    pub fn clear_state(&mut self) {
    }
}

type BlockId = usize;

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
struct Blip {
    pos: Vec3,
    move_progress: usize,
}

const MOVE_TICKS_PER_NODE: usize = 10;

#[derive(PartialEq, Eq, Clone, Debug)]
struct Blocks {
    ids: Grid3<Option<BlockId>>,
    data: VecOption<Block>,
}

impl Blocks {
    pub fn new(size: Vec3) -> Blocks {
        Blocks {
            ids: Grid3::new(size),
            data: VecOption::new(),
        }
    }

    pub fn at_pos(&self, p: Vec3) -> Option<&Block> {
        self
            .ids
            .get(p)
            .and_then(|id| id.as_ref())
            .map(|&id| &self.data[id])
    }

    pub fn clear_state(&mut self) {
        for (_, block) in self.data.iter_mut() {
            block.clear_state();
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
struct Machine {
    blocks: Blocks,
    blips: VecOption<Blip>,
}

impl Machine {
    pub fn new(size: Vec3) -> Machine {
        Machine {
            blocks: Blocks::new(size),
            blips: VecOption::new(),
        }
    }

    pub fn clear_state(&mut self) {
        self.blocks.clear_state();
        self.blips.clear();
    }

    pub fn run(&mut self) {
        let mut blips_to_remove = Vec::<BlockId>::new();

        for (blip_id, blip) in self.blips.iter_mut() {
            blip.move_progress += 1;

            let mut move_dir = None;

            if blip.move_progress == MOVE_TICKS_PER_NODE {
                match self.blocks.at_pos(blip.pos) {
                    Some(Block::Pipe { from: _, to }) => {
                        move_dir = Some(*to);
                    },
                    Some(Block::Switch(dir)) => {
                        move_dir = Some(*dir);
                    },
                    None => {
                        if blip.pos.z == 0 {
                            blips_to_remove.push(blip_id);
                            continue;
                        } else {
                            move_dir = Some(Dir3(Axis3::Z, Sign::Neg));
                        }
                    }
                    _ => (),
                }
            }

            if let Some(move_dir) = move_dir {
                blip.pos += move_dir.to_vector();

                let remove = match self.blocks.at_pos(blip.pos) {
                    Some(Block::Pipe { from, to: _ }) => *from != move_dir.invert(),
                    Some(Block::Switch(dir)) => *dir != move_dir,
                    Some(Block::Solid) => true,
                    _ => false,
                };

                if remove {
                    blips_to_remove.push(blip_id);
                }
            }
        }

        for blip_id in blips_to_remove {
            self.blips.remove(blip_id);
        }
    }
}
