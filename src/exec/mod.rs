pub mod view;

use std::mem;

use log::debug;

use crate::machine::grid::{Dir3, Grid3, Point3};
use crate::machine::{Block, BlockIndex, Machine, PlacedBlock, BlipKind};
use crate::util::vec_option::VecOption;

pub use view::ExecView;

const MOVE_TICKS_PER_NODE: usize = 10;

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct BlipMovement {
    pub dir: Dir3,
    pub progress: usize,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct Blip {
    pub kind: BlipKind,
    pub pos: Point3,
    pub movement: Option<BlipMovement>,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Default)]
pub struct WindState {
    pub wind_in: [bool; Dir3::NUM_INDICES],
}

impl WindState {
    pub fn wind_in(&self, dir: Dir3) -> bool {
        self.wind_in[dir.to_index()]
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Default)]
pub struct BlipState {
    pub blip: Option<Blip>,
}

pub struct Exec {
    machine: Machine,
    blips: VecOption<Blip>,

    /// Wind state for each block, indexed by BlockIndex
    wind_state: Vec<WindState>,

    /// Wind state from the previous tick, used for double
    /// buffering
    old_wind_state: Vec<WindState>,

    /// Blip state for each block, indexed by BlockIndex
    blip_state: Vec<BlipState>,
}

impl Exec {
    pub fn new(mut machine: Machine) -> Exec {
        // Make the machine's blocks contiguous in memory.
        machine.gc();

        let wind_state = Exec::initial_block_state(&machine);
        let old_wind_state = wind_state.clone();
        let blip_state = Exec::initial_block_state(&machine);

        Exec {
            machine,
            blips: VecOption::new(),
            wind_state,
            old_wind_state,
            blip_state,
        }
    }

    pub fn update(&mut self) {
        mem::swap(&mut self.wind_state, &mut self.old_wind_state);

        for (block_index, (block_pos, placed_block)) in self.machine.block_data.iter() {
            Self::update_block_wind_state(
                block_index,
                block_pos,
                placed_block,
                &self.machine.block_ids,
                &self.old_wind_state,
                &mut self.wind_state,
            );
        }

        for (block_index, (block_pos, placed_block)) in self.machine.block_data.iter_mut() {
            Self::update_block_blip_state(
                block_index,
                block_pos,
                placed_block,
                &self.machine.block_ids,
                &self.wind_state,
                &mut self.blip_state,
            );
        }

        for (index, _) in self.machine.block_data.iter_mut() {
            self.old_wind_state[index] = self.wind_state[index];
        }
    }

    fn update_block_wind_state(
        block_index: usize,
        block_pos: &Point3,
        placed_block: &PlacedBlock,
        block_ids: &Grid3<Option<BlockIndex>>,
        old_wind_state: &[WindState],
        wind_state: &mut Vec<WindState>,
    ) {
        debug!(
            "have {:?} with {:?}",
            placed_block.block, old_wind_state[block_index]
        );

        match placed_block.block {
            Block::WindSource => {
                for dir in &Dir3::ALL {
                    let neighbor_pos = *block_pos + dir.to_vector();
                    let neighbor_index = block_ids.get(&neighbor_pos);

                    if let Some(Some(neighbor_index)) = neighbor_index {
                        wind_state[*neighbor_index].wind_in[dir.invert().to_index()] = true;
                    }
                }
            }
            _ => {
                debug!("wind holes: {:?}", placed_block.wind_holes());

                let any_in = placed_block
                    .wind_holes()
                    .iter()
                    .map(|dir| old_wind_state[block_index].wind_in(*dir))
                    .any(|b| b);

                debug!("in flow: {}", any_in);

                for dir in &placed_block.wind_holes() {
                    let neighbor_pos = *block_pos + dir.to_vector();

                    if let Some(Some(neighbor_index)) = block_ids.get(&neighbor_pos) {
                        let neighbor_in_flow = if any_in {
                            !old_wind_state[block_index].wind_in[dir.to_index()]
                        } else {
                            false
                        };

                        debug!("flow to {:?}: {}", dir, neighbor_in_flow);

                        wind_state[*neighbor_index].wind_in[dir.invert().to_index()] =
                            neighbor_in_flow;
                    }
                }
            }
        }
    }

    fn update_block_blip_state(
        block_index: usize,
        block_pos: &Point3,
        placed_block: &mut PlacedBlock,
        block_ids: &Grid3<Option<BlockIndex>>,
        wind_state: &[WindState],
        blip_state: &mut Vec<BlipState>,
    ) {
    }

    fn initial_block_state<T: Default + Copy>(machine: &Machine) -> Vec<T> {
        // We assume that the machine's blocks are contiguous in memory, so that
        // we can store wind state as a Vec, instead of wasting memory or cycles
        // on VecOption while executing.
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
