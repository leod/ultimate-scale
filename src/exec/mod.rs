pub mod anim;
pub mod level_progress;
pub mod play;
#[cfg(test)]
mod tests;
pub mod view;
pub mod neighbors;

use std::iter;

use log::{debug, info};
use rand::Rng;

use crate::machine::grid::{Axis3, Dir3, Grid3, Point3, DirMap3};
use crate::machine::{level, BlipKind, Block, BlockIndex, Machine, PlacedBlock, TickNum};
use crate::util::vec_option::VecOption;

use neighbors::NeighborMap;

pub use play::TickTime;
pub use view::ExecView;

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct BlipMovement {
    pub dir: Dir3,
    pub progress: usize,
}

/// Ways that blips can enter live.
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum BlipSpawnMode {
    Ease,
    Quick,
    LiveToDie,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum BlipStatus {
    Spawning(BlipSpawnMode),
    Existing,
    Dying,
}

impl BlipStatus {
    fn is_dead(self) -> bool {
        BlipStatus::Spawning(BlipSpawnMode::LiveToDie) => true,
        BlipStatus::Dying => true,
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct Blip {
    /// Blip kind.
    pub kind: BlipKind,

    /// The blip's current position on the grid.
    pub pos: Point3,

    /// The direction in which the blip moved last tick, if any.
    pub move_dir: Option<Dir3>,

    /// Status. Used mostly for visual purposes. Blips marked as Dying will
    /// be removed at the start of the next tick.
    pub status: BlipStatus,
}

pub type BlipIndex = usize;

pub fn flow_nowhere() -> DirMap<bool> {
    Default::default()
}

pub fn flow_everywhere() -> DirMap<bool> {
    DirMap([true; Dir3::NUM_INDICES])
}

pub fn flow_only(dir: Dir3) -> DirMap<bool> {
    let mut flow = Self::nowhere();
    flow[except] = true;

    flow
}

pub fn flow_everywhere_except(except: Dir3) -> DirMap<bool> {
    let mut flow = Self::everywhere();
    flow[except] = false;

    flow
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum LevelStatus {
    Running,
    Completed,
    Failed,
}

pub type Activation = Option<BlipIndex>;

struct BlocksState {
    wind_out: Vec<DirMap<bool>>,
    next_wind_out: Vec<DirMap<bool>>,

    prev_activation: Vec<Activation>,
    activation: Vec<Activation>,
    next_activation: Vec<Activation>,

    activation: Vec<Activation>,
    blip_count: Vec<usize>,
}

impl BlocksState {
    fn new_initial(machine: &Machine) -> Self {
        // We assume that the machine's blocks are contiguous in memory, so that
        // we can store block state as a Vec, instead of wasting memory or
        // cycles on VecOption while executing.
        assert!(machine.is_contiguous());
        
        State {
            wind: vec![DirMap::default(); machine.num_blocks()],
            next_wind: vec![DirMap::default(); machine.num_blocks()],
            activation: vec![Activation::default(); machine.num_blocks()],
            blip_count: vec![0; machine.num_blocks()],
        }
    }
}

pub struct Exec {
    cur_tick: TickNum,

    machine: Machine,
    neighbor_map: NeighborMap,

    inputs_outputs: Option<level::InputsOutputs>,
    level_status: LevelStatus,

    blips: VecOption<Blip>,
    blocks: BlocksState,
}

impl Exec {
    pub fn new<R: Rng + ?Sized>(mut machine: Machine, rng: &mut R) -> Exec {
        // Make the machine's blocks contiguous in memory.
        machine.gc();

        let neighbor_map = NeighborMap::new_from_machine(&machine);
        let inputs_outputs = machine
            .level
            .as_ref()
            .map(|level| level.spec.gen_inputs_outputs(rng));

        if let Some(inputs_outputs) = inputs_outputs.as_ref() {
            initialize_inputs_outputs(inputs_outputs, &mut machine);
        }

        Exec {
            cur_tick: 0,
            machine,
            neighbor_map,
            level_status: LevelStatus::Running,
            inputs_outputs,
            blips: VecOption::new(),
            blocks: BlocksState::new_initial(&machine),
        }
    }

    pub fn machine(&self) -> &Machine {
        &self.machine
    }

    pub fn level_status(&self) -> LevelStatus {
        self.level_status
    }

    pub fn inputs_outputs(&self) -> Option<&level::InputsOutputs> {
        self.inputs_outputs.as_ref()
    }

    pub fn wind(&self) -> &[WindState] {
        &self.blocks.wind
    }

    pub fn next_wind(&self) -> &[WindState] {
        &self.blocks.next_wind
    }

    pub fn blips(&self) -> &VecOption<Blip> {
        &self.blips
    }

    pub fn update(&mut self) {
        self.check_consistency();

        std::mem::swap(&mut self.blocks.wind, &mut self.blocks.next_wind);

        // Perform blip movement, as it was defined in the previous update.
        for (blip_index, blip) in self.blips {
            if let Some(move_dir) = blip.move_dir {
                blip.pos = blip.pos + move_dir.to_vector();

                if let Some(block_index) = self.machine.blocks.indices.get(blip.pos).flatten() {
                    
                }
            }
        }

        // Remove dead blips.
        self.blips.retain(|blip| !blip.state.is_dead());

        // Spawn and move wind
        for block_index in 0..self.machine.num_blocks() {
            update_wind_state(
                block_index,
                &self.machine,
                &self.old_state,
                &mut self.wind_state,
            );
        }

        // Determine blip movement directions.

        // Run effects of blocks that were activated in the last tick
        for (block_index, (_, placed_block)) in self.machine.blocks.data {
            if let Some(blip_kind) = self.old_state.activated[block_index] {
                run_activated_block(
                    &placed_block.block,
                    blip_kind,
                    &mut self.blips,
                );
            }
        }

        // Check for 

        self.check_consistency();

        self.cur_tick += 1;
    }
}

fn advect_wind(
    block_index: usize,
    machine: &Machine,
    neighbor_map: &NeighborMap,
    prev_activation: &[Activation],
    wind_out: &[DirMap<bool>],
) -> DirMap<bool> {
    let (block_pos, ref placed_block) = &machine.blocks.data[block_index];

    match placed_block.block {
        Block::WindSource => flow_everywhere(),
        Block::BlipWindSource {
            button_dir,
        } => {
            if prev_activation[block_index].is_some() {
                flow_everywhere_except(button_dir)
            } else {
                flow_nowhere()
            }
        }
        Block::Input {
            out_dir,
            activated,
            ..
        } => {
            let _active = activated.map_or(false, |input| match input {
                level::Input::Blip(_) => true,
            });

            // For now, we'll set Input blocks to always spawn wind.
            // For the future, it might be interesting to spawn wind only
            // when active -- this will also allow interpreting the Option
            // in InputsOutputs. Note however, that currently this would
            // lead to a gap inbetween each spawned blip, since it takes
            // some time for the wind to reach from the Input center to the
            // spawned blip.
            let active = true;

            if active {
                flow_only(out_dir)
            } else {
                flow_nowhere()
            }
        }
        _ => {
            // Check if we got any wind in flow from our neighbors in the
            // old state
            let mut any_in = false;
            let mut wind_in = DirMap3::default();

            for (dir, neighbor_index) in neighbor_map.iter(block_index) {
            }

            for &dir in &placed_block.wind_holes_in() {
                let neighbor_pos = *block_pos + dir.to_vector();

                wind_in[dir.to_index()] =
                    if let Some(neighbor_index) = block_ids.get(&neighbor_pos).flatten() {
                        let neighbor_had_wind =
                            old_state.wind[*neighbor_index].wind_out(dir.invert());

                        let neighbor_has_hole =
                            block_data[*neighbor_index].1.has_wind_hole_out(dir.invert());

                        neighbor_had_wind && neighbor_has_hole
                    } else {
                        false
                    };

                any_in = any_in || wind_in[dir.to_index()];
            }

            // Forward in flow to our outgoing wind hole directions
            let mut result = flow_nowhere();

            if any_in {
                for (neighbor_dir, neighbor_index) in neighbor_map.iter(block_index) {
                    if !wind_in[dir]
                }
            }

            for &dir in &placed_block.wind_holes_out() {
                wind_state[block_index].set_wind_out(
                    dir,
                    any_in && !wind_in[dir.to_index(),
                );
            }
        }
    }
}

fn spawn_blip(
    blips: &mut VecOption<Blip>,
)

fn run_activated_block(
    block: &Block,
    blip_kind: BlipKind,
    blips: &mut VecOption<Blip>,
) {
    match block {
        BlipDuplicator {
            out_dirs: (out_dir_1, out_dir_2),
            ..
        } => {
        }
    }
}

fn check_output(block_data: &mut VecOption<(Point3, PlacedBlock)>) -> LevelStatus {
    let mut failed = false;
    let mut completed = true;

    for (_, (_, block)) in block_data.iter_mut() {
        if let Block::Output {
            ref mut outputs,
            ref activated,
            failed: ref mut output_failed,
            ..
        } = block.block
        {
            // The last element of `outputs` is the next expected output.
            // Note that the last element will be popped at the start of
            // the next tick in `update_block`.
            let expected = outputs.last().copied();

            let (block_failed, block_completed) = match (expected, activated) {
                (Some(expected), Some(activated)) => {
                    (expected != *activated, outputs.len() == 1)
                }
                (Some(_), None) => (false, false),
                (None, Some(_)) => (true, false),
                (None, None) => (false, true),
            };

            if block_failed {
                // Remember failure status for visualization.
                *output_failed = true;
            }

            failed = failed || block_failed;
            completed = completed && block_completed;
        }
    }

    if failed {
        info!("Level failed");
        LevelStatus::Failed
    } else if completed {
        info!("Level completed");
        LevelStatus::Completed
    } else {
        LevelStatus::Running
    }
}

fn initialize_inputs_outputs(inputs_outputs: &level::InputsOutputs, machine: &mut Machine) {
    for (i, input_spec) in inputs_outputs.inputs.iter().enumerate() {
        for (_, (_, block)) in machine.blocks.data.iter_mut() {
            match &mut block.block {
                Block::Input { index, inputs, .. } if *index == i => {
                    // We reverse the inputs so that we can use Vec::pop
                    // during execution to get the next input.
                    *inputs = input_spec.iter().copied().rev().collect();

                    // Block::Input index is assumed to be unique
                    break;
                }
                _ => (),
            }
        }
    }

    for (i, output_spec) in inputs_outputs.outputs.iter().enumerate() {
        for (_, (_, block)) in machine.blocks.data.iter_mut() {
            match &mut block.block {
                Block::Output { index, outputs, .. } if *index == i => {
                    // We reverse the outputs so that we can use Vec::pop
                    // during execution to get the next expected output.
                    *outputs = output_spec.iter().copied().rev().collect();

                    // Block::Output index is assumed to be unique
                    break;
                }
                _ => (),
            }
        }
    }
}

