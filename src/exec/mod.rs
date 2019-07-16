pub mod view;

use std::mem;

use log::{warn, debug};

use crate::util::vec_option::VecOption;
use crate::machine::grid::{Point3, Axis3, Sign, Dir3, Grid3};
use crate::machine::{Block, BlockIndex, Machine};

pub use view::ExecView;

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

impl WindState {
    pub fn flow_out(&self, dir: &Dir3) -> bool {
        self.flow_out[dir.to_index()]
    }
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
        machine.gc();

        let wind_state = Exec::initial_wind_state(&machine);
        let old_wind_state = wind_state.clone();

        Exec {
            machine,
            blips: VecOption::new(),
            wind_state,
            old_wind_state: old_wind_state,
        }
    }

    pub fn update(&mut self) {
        mem::swap(&mut self.wind_state, &mut self.old_wind_state);

        for (index, (block_pos, placed_block)) in self.machine.block_data.iter_mut() {
            let wind_state = &mut self.wind_state[index];

            debug!("have {:?} with {:?}", placed_block.block, wind_state);
            
            match placed_block.block {
                Block::Solid => {
                    for index in 0 .. Dir3::NUM_INDICES {
                        wind_state.flow_out[index] = true;
                    }
                }
                Block::PipeXY => {
                    let in_dir_a = placed_block.rotated_dir(Dir3(Axis3::Y, Sign::Neg));
                    let in_dir_b = placed_block.rotated_dir(Dir3(Axis3::Y, Sign::Pos));

                    let in_pos_a = *block_pos + in_dir_a.to_vector();
                    let in_pos_b = *block_pos + in_dir_b.to_vector();

                    debug!("neighbor dirs: {:?} {:?}", in_dir_a, in_dir_b);

                    if let Some(Some(in_block_id)) = self.machine.block_ids.get(&in_pos_a) {
                        let in_wind_state = &self.old_wind_state[*in_block_id];
                        wind_state.flow_out[in_dir_b.to_index()] = in_wind_state.flow_out(&in_dir_b);
                    }

                    if let Some(Some(in_block_id)) = self.machine.block_ids.get(&in_pos_b) {
                        let in_wind_state = &self.old_wind_state[*in_block_id];
                        wind_state.flow_out[in_dir_a.to_index()] = in_wind_state.flow_out(&in_dir_a);
                    }
                }
                _ => warn!("Wind flow of {:?} is unimplemented!", placed_block.block),
            }
        }

        for (index, _) in self.machine.block_data.iter_mut() {
            self.old_wind_state[index] = self.wind_state[index];
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
