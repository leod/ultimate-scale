use crate::util::vec_option::VecOption;

use crate::machine::grid::Vec3;
use crate::machine::{Axis3, Sign, Dir3, Block, BlockId, Machine};

const MOVE_TICKS_PER_NODE: usize = 10;

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct Blip {
    pub pos: Vec3,
    pub move_progress: usize,
}

struct Exec {
    machine: Machine,
    blips: VecOption<Blip>,
}

impl Exec {
    pub fn new(machine: Machine) -> Exec {
        Exec {
            machine,
            blips: VecOption::new(),
        }
    }

    pub fn update(&mut self) {
        let mut blips_to_remove = Vec::<BlockId>::new();

        for (blip_id, blip) in self.blips.iter_mut() {
            blip.move_progress += 1;

            let mut move_dir = None;

            if blip.move_progress == MOVE_TICKS_PER_NODE {
                match self.machine.blocks.at_pos(blip.pos) {
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

                let remove = match self.machine.blocks.at_pos(blip.pos) {
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
