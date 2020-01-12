pub mod anim;
pub mod level_progress;
pub mod neighbors;
pub mod play;
#[cfg(test)]
mod tests;
pub mod view;

use std::convert::identity;
use std::iter;

use log::{debug, info};
use rand::Rng;

use crate::machine::grid::{Axis3, Dir3, DirMap3, Grid3, Point3};
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
        match self {
            BlipStatus::Spawning(BlipSpawnMode::LiveToDie) => true,
            BlipStatus::Dying => true,
            _ => false,
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct Blip {
    /// Blip kind.
    pub kind: BlipKind,

    /// The blip's current position on the grid.
    pub pos: Point3,

    /// The last direction that this blip moved in.
    pub last_move_dir: Dir3,

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
    next_blip_count: Vec<usize>,
}

impl BlocksState {
    fn new_initial(machine: &Machine) -> Self {
        // We assume that the machine's blocks are contiguous in memory, so that
        // we can store block state as a Vec, instead of wasting memory or
        // cycles on VecOption while executing.
        assert!(machine.is_contiguous());

        State {
            wind_out: vec![DirMap::default(); machine.num_blocks()],
            next_wind_out: vec![DirMap::default(); machine.num_blocks()],
            activation: vec![Activation::default(); machine.num_blocks()],
            next_blip_count: vec![0; machine.num_blocks()],
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
        // 1) Perform blip movement, as it was defined in the previous update.
        for (blip_index, blip) in self.blips {
            if let Some(move_dir) = blip.move_dir {
                blip.pos = blip.pos + move_dir.to_vector();
                blip.last_move_dir = move_dir;
                blip.move_dir = None;
            }
        }

        // 2) Remove dead blips.
        self.blips.retain(|blip| !blip.state.is_dead());

        // 3) Spawn and move wind.
        std::mem::swap(&mut self.blocks.wind, &mut self.blocks.next_wind);

        for block_index in 0..self.machine.num_blocks() {
            self.blocks.next_wind_out[block_index] = spawn_or_advect_wind(
                block_index,
                &self.machine,
                &self.neighbor_map,
                &self.blocks.wind_out,
                &self.blocks.prev_activation,
            );
        }

        // 4) Determine blip movement directions.
        for block_index in 0..self.machine.num_blocks() {
            self.blocks.next_blip_count[block_index] = 0;
        }

        for (_blip_index, blip) in self.blips.iter_mut() {}

        // 5) Run effects of blocks that were activated in this tick.
        for (block_index, (block_pos, placed_block)) in self.machine.iter_blocks() {
            if let Some(blip_kind) = self.blocks.activation[block_index] {
                run_activated_block(*block_pos, &placed_block.block, blip_kind, &mut self.blips);
            }
        }

        self.cur_tick += 1;
    }
}

fn spawn_or_advect_wind(
    block_index: usize,
    machine: &Machine,
    neighbor_map: &NeighborMap,
    wind_out: &[DirMap<bool>],
    prev_activation: &[Activation],
) -> DirMap<bool> {
    let block = machine.block_at_index(block_index);
    match block {
        Block::WindSource => flow_everywhere(),
        Block::BlipWindSource { button_dir } => {
            if prev_activation[block_index].is_some() {
                flow_everywhere_except(button_dir)
            } else {
                flow_nowhere()
            }
        }
        Block::Input { out_dir, .. } => flow_only(out_dir),
        _ => {
            // Check if we got any wind in flow from our neighbors in the
            // old state
            let block_wind_in = neighbor_map[block_index].map(|(dir, neighbor_index)| {
                neighbor_index.map_or(false, |neighbor_index| {
                    block.has_wind_hole_in(dir) && wind_out[neighbor_index][dir.invert()]
                })
            });

            if block_wind_in.values().any(identity) {
                // Forward in flow to our outgoing wind hole directions
                neighbor_map[block_index].map(|(dir, neighbor_index)| {
                    neighbor_index.map_or(true, |neighbor_index| {
                        block.has_wind_hole_out(dir) && !block_wind_in[dir]
                    })
                })
            } else {
                flow_nowhere()
            }
        }
    }
}

fn find_dir_ccw_xy(initial_dir: Dir3, f: impl Fn(Dir3) -> bool) -> Option<Dir3> {
    iter::successors(Some(initial_dir), |dir| Some(dir.rotated_ccw_xy()))
        .take(4)
        .find(f)
}

fn blip_move_dir(
    blip: &Blip,
    machine: &Machine,
    neighbor_map: &NeighborMap,
    next_wind_out: &[DirMap<bool>],
) -> Option<Dir3> {
    let block = &machine.get(blip.pos).block;

    let block_move_out = neighbor_map[block_index].map(|(dir, neighbor_index)| {
        neighbor_index.map_or(false, |neighbor_index| {
            next_wind_out[block_index][dir]
                && block.has_move_hole(dir)
                && machine
                    .block_at_index(neighbor_index)
                    .has_move_hole(dir.invert())
        });
    });
    let block_wind_in = neighbor_map[block_index].map(|(dir, neighbor_index)| {
        neighbor_index.map_or(false, |neighbor_index| {
            block.has_wind_hole_in(dir) && next_wind_out[neighbor_index][dir.invert()]
        })
    });

    let num_move_out = block_move_out.values().map(|flow| flow as usize).sum();
    let num_wind_in = block_wind_in.values().map(|flow| flow as usize).sum();

    match num_wind_in {
        1 => {
            let wind_in_dir = wind_in.iter().find(|dir, flow| flow).0;

            find_dir_ccw_xy(wind_in_dir.invert(), |dir| {
                !wind_in[dir] && block_move_out[dir]
            })
        }
        3 => {
            let all_wind_in_xy = block_wind_in
                .iter()
                .map(|(dir, flow)| !flow || dir.0 != Axis3::Z)
                .all();

            if all_wind_in_xy {
                Dir3::ALL_XY
                    .iter()
                    .cloned()
                    .find(|dir| !wind_in[dir] && block_move_out[dir])
            } else {
                // TODO: I don't think this can actually happen.
                None
            }
        }
        _ => None,
    }
}

fn run_activated_block(
    block_pos: &Point3,
    block: &Block,
    blip_kind: BlipKind,
    blips: &mut VecOption<Blip>,
) {
    match block {
        BlipDuplicator { out_dirs, .. } => {
            for &dir in &[out_dir.0, out_dirs.1] {
                blips.add(Blip {
                    kind: blip_kind,
                    pos: block_pos,
                    last_move_dir: dir,
                    move_dir: Some(dir),
                    status: BlipStatus::Spawning(BlipSpawnMode::Quick),
                });
            }
        }
        _ => (),
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
