pub mod view;

use log::debug;

use crate::machine::grid::{Dir3, Grid3, Point3};
use crate::machine::{BlipKind, Block, BlockIndex, Machine, PlacedBlock};
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

pub type BlipIndex = usize;

#[derive(PartialEq, Eq, Clone, Copy, Debug, Default)]
pub struct BlipState {
    pub blip_index: Option<BlipIndex>,
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

    /// Blip state from the previous tick
    old_blip_state: Vec<BlipState>,
}

impl Exec {
    pub fn new(mut machine: Machine) -> Exec {
        // Make the machine's blocks contiguous in memory.
        machine.gc();

        let wind_state = Self::initial_block_state(&machine);
        let old_wind_state = wind_state.clone();
        let blip_state = Self::initial_block_state(&machine);
        let old_blip_state = blip_state.clone();

        Exec {
            machine,
            blips: VecOption::new(),
            wind_state,
            old_wind_state,
            blip_state,
            old_blip_state,
        }
    }

    pub fn machine(&self) -> &Machine {
        &self.machine
    }

    pub fn wind_state(&self) -> &[WindState] {
        &self.wind_state
    }

    pub fn blips(&self) -> &VecOption<Blip> {
        &self.blips
    }

    pub fn update(&mut self) {
        for index in 0..self.wind_state.len() {
            self.old_wind_state[index] = self.wind_state[index];
        }

        for index in 0..self.blip_state.len() {
            self.old_blip_state[index] = self.blip_state[index];

            //self.old_blip_state[index] = self.blip_state[index];
            self.blip_state[index].blip_index = None;
        }

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

        Self::update_blips(
            &self.machine.block_ids,
            &self.wind_state,
            &self.old_blip_state,
            &mut self.blip_state,
            &mut self.machine.block_data,
            &mut self.blips,
        );

        for (block_index, (block_pos, placed_block)) in self.machine.block_data.iter_mut() {
            Self::update_block_blip_state(
                block_index,
                block_pos,
                placed_block,
                &self.machine.block_ids,
                &self.wind_state,
                &mut self.blip_state,
                &mut self.blips,
            );
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
            "wind: {:?} with {:?}",
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
                let any_in = placed_block
                    .wind_holes()
                    .iter()
                    .map(|dir| old_wind_state[block_index].wind_in(*dir))
                    .any(|b| b);

                debug!(
                    "wind holes {:?}, rot {}",
                    placed_block.wind_holes(),
                    placed_block.rotation_xy
                );
                for dir in &placed_block.wind_holes() {
                    let neighbor_pos = *block_pos + dir.to_vector();

                    debug!("check wind guy {:?} at {:?}", block_pos, neighbor_pos);
                    if let Some(Some(neighbor_index)) = block_ids.get(&neighbor_pos) {
                        let neighbor_in_flow = if any_in {
                            !old_wind_state[block_index].wind_in[dir.to_index()]
                        } else {
                            false
                        };

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
        blips: &mut VecOption<Blip>,
    ) {
        let dir_x_pos = placed_block.rotated_dir_xy(Dir3::X_POS);

        match placed_block.block {
            Block::BlipSpawn {
                kind,
                ref mut num_spawns,
            } => {
                let do_spawn = num_spawns.map_or(true, |n| n > 0);

                if do_spawn {
                    let output_pos = *block_pos + dir_x_pos.to_vector();

                    if let Some(Some(output_index)) = block_ids.get(&output_pos) {
                        if blip_state[*output_index].blip_index.is_none() {
                            debug!("spawning blip at {:?}", output_pos);

                            let blip = Blip {
                                kind: kind,
                                pos: output_pos,
                            };
                            blip_state[*output_index].blip_index = Some(blips.add(blip));
                        }
                    }

                    *num_spawns = num_spawns.map_or(None, |n| Some(n - 1));
                }
            }
            _ => {}
        }
    }

    fn update_blips(
        block_ids: &Grid3<Option<BlockIndex>>,
        wind_state: &[WindState],
        old_blip_state: &[BlipState],
        blip_state: &mut Vec<BlipState>,
        block_data: &mut VecOption<(Point3, PlacedBlock)>,
        blips: &mut VecOption<Blip>,
    ) {
        let mut remove_indices = Vec::new();

        for (blip_index, blip) in blips.iter_mut() {
            let block_index = block_ids.get(&blip.pos);

            if let Some(Some(block_index)) = block_index {
                debug!(
                    "blip at {:?}: {:?} vs {:?}",
                    blip.pos, old_blip_state[*block_index].blip_index, blip_index,
                );
                assert!(old_blip_state[*block_index].blip_index == Some(blip_index));
                assert!(block_data[*block_index].0 == blip.pos);

                let block = &mut block_data[*block_index].1;

                // To determine movement, check in flow of neighboring blocks
                let out_dir = Dir3::ALL.iter().find(|dir| {
                    // TODO: At some point, we'll need to precompute neighbor
                    //       indices.

                    let neighbor_index = block_ids.get(&(blip.pos + dir.to_vector()));
                    let neighbor_wind_in = if let Some(Some(neighbor_index)) = neighbor_index {
                        wind_state[*neighbor_index].wind_in(dir.invert())
                    } else {
                        false
                    };

                    neighbor_wind_in && block.has_move_hole(**dir)
                });

                let new_pos = if let Some(out_dir) = out_dir {
                    // Apply effects of leaving the current block
                    match block.block.clone() {
                        Block::PipeSplitXY { open_move_hole_y } => {
                            block.block = Block::PipeSplitXY {
                                open_move_hole_y: open_move_hole_y.invert(),
                            };
                        }
                        _ => (),
                    }

                    blip.pos + out_dir.to_vector()
                } else {
                    blip.pos
                };

                let new_block_index = block_ids.get(&new_pos);

                if let Some(Some(new_block_index)) = new_block_index {
                    blip.pos = new_pos;
                    debug!(
                        "moving blip {} from {:?} to {:?}",
                        blip_index, blip.pos, new_pos
                    );

                    if let Some(new_block_blip_index) = blip_state[*new_block_index].blip_index {
                        // We cannot have two blips in the same block. Note
                        // that if more than two blips move into the same
                        // block, the same blip will be added multiple times
                        // into `remove_indices`. This is fine, since we don't
                        // spawn any blips in this function, so the indices
                        // stay valid.
                        debug!(
                            "{} bumped into {}, removing",
                            blip_index, new_block_blip_index
                        );
                        remove_indices.push(blip_index);
                        remove_indices.push(new_block_blip_index);
                    } else {
                        blip_state[*new_block_index].blip_index = Some(blip_index);
                    }
                } else {
                    // Out of bounds
                    remove_indices.push(blip_index);
                }
            } else {
                // Out of bounds.
                // TODO: Can this happen?
                remove_indices.push(blip_index);
            };
        }

        for remove_index in remove_indices {
            if blips.contains(remove_index) {
                let pos = blips[remove_index].pos;

                debug!("removing blip {} at pos {:?}", remove_index, pos);

                if let Some(Some(block_index)) = block_ids.get(&pos) {
                    blip_state[*block_index].blip_index = None;
                }

                blips.remove(remove_index);
            }
        }
    }

    fn initial_block_state<T: Default + Copy>(machine: &Machine) -> Vec<T> {
        // We assume that the machine's blocks are contiguous in memory, so that
        // we can store wind state as a Vec, instead of wasting memory or cycles
        // on VecOption while executing.
        assert!(machine.is_contiguous());

        vec![Default::default(); machine.num_blocks()]
    }
}
