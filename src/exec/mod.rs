pub mod view;

use std::mem;

use crate::util::vec_option::VecOption;

pub use view::ExecView;

use crate::machine::grid::{Point3, Axis3, Sign, Dir3, Grid3};
use crate::machine::{Block, BlockIndex, Machine};

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

pub struct Exec {
    machine: Machine,
    blips: VecOption<Blip>,
    
    /// Wind state for each block, indexed by BlockId
    wind_state: Vec<WindState>,

    /// Wind state from the previous tick, used for double
    /// buffering
    old_wind_state: Vec<WindState>,
}

impl Exec {
    pub fn new(mut machine: Machine) -> Exec {
        let wind_state = Exec::initial_wind_state(&machine);
        let old_wind_state = wind_state.clone();

        machine.gc();

        Exec {
            machine,
            blips: VecOption::new(),
            wind_state,
            old_wind_state: old_wind_state,
        }
    }

    pub fn update(&mut self) {
        mem::swap(&mut self.wind_state, &mut self.old_wind_state);

        for (index, (block_pos, placed_block)) in self.machine.iter_blocks_mut() {
            let wind_state = &mut self.wind_state[index];
            
            match placed_block.block {
                Block::Solid => {
                    for index in 0 .. Dir3::NUM_INDICES {
                        wind_state.flow_out[index] = true;
                    }
                }
                Block::PipeXY => {
                   let in_dir = placed_block.rotate_dir(&Dir3(Axis3::X, Sign::Neg));
                   let in_pos = *block_pos + in_dir.to_vector();

                   /*if let Some(in_block) = self.machine.get_block_at_pos(&in_pos) {
                    
                   }*/
                }
                _ => unimplemented!(),
            }
        }
    }

    fn initial_wind_state(machine: &Machine) -> Vec<WindState> {
        // We assume that the machine's blocks are contiguous in memory
        assert!(machine.is_contiguous());

        vec![Default::default(); machine.num_blocks()]
    }

    pub fn machine(&self) -> &Machine {
        &self.machine
    }

    pub fn wind_state(&self) -> &Vec<WindState> {
        &self.wind_state
    }
}
