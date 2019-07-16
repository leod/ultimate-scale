use crate::util::vec_option::VecOption;

use crate::machine::grid::{Point3, Axis3, Sign, Dir3, Grid3};
use crate::machine::{Block, BlockId, Machine};

const MOVE_TICKS_PER_NODE: usize = 10;

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct Blip {
    pub pos: Point3,
    pub move_progress: usize,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Default)]
pub struct WindState {
    pub flow_out: [bool; Dir3::NUM_INDICES],
}

struct Exec {
    machine: Machine,
    blips: VecOption<Blip>,
    wind_state: Grid3<Option<WindState>>,
}

impl Exec {
    pub fn new(machine: Machine) -> Exec {
        let wind_state = Exec::initial_wind_state(&machine);

        Exec {
            machine,
            blips: VecOption::new(),
            wind_state,
        }
    }

    pub fn update(&mut self) {
        for (block_pos, placed_block) in self.machine.iter_blocks_mut() {
            //let x = self.machine.get_block(&block_pos);
        }
    }

    fn initial_wind_state(machine: &Machine) -> Grid3<Option<WindState>> {
        Grid3::new(machine.size())
    }
}
