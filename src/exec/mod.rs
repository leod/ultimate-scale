pub mod anim;
pub mod level_progress;
pub mod neighbors;
pub mod play;
#[cfg(test)]
mod tests;
pub mod view;

use std::cmp;
use std::convert::identity;
use std::iter;
use std::mem;

use log::{debug, info};
use rand::Rng;

use crate::machine::grid::{Axis3, Dir3, DirMap3, Grid3, Point3, Vector3};
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
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum BlipStatus {
    Spawning(BlipSpawnMode),
    Existing,
    LiveToDie,
    Dying,
}

impl BlipStatus {
    fn is_spawning(self) -> bool {
        if let BlipStatus::Spawning(_) = self {
            true
        } else {
            false
        }
    }

    fn is_dead(self) -> bool {
        match self {
            BlipStatus::Dying => true,
            BlipStatus::LiveToDie => true,
            _ => false,
        }
    }

    fn kill(self) -> Self {
        match self {
            BlipStatus::Spawning(_) => BlipStatus::LiveToDie,
            BlipStatus::Existing => BlipStatus::Dying,
            BlipStatus::LiveToDie => BlipStatus::LiveToDie,
            BlipStatus::Dying => BlipStatus::Dying,
        }
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

impl Blip {
    fn new(kind: BlipKind, pos: Point3, move_dir: Option<Dir3>, spawn_mode: BlipSpawnMode) -> Self {
        Blip {
            kind,
            pos,
            move_dir,
            status: BlipStatus::Spawning(spawn_mode),
        }
    }

    fn next_pos(&self) -> Point3 {
        self.pos + self.move_dir.map_or(Vector3::zero(), |dir| dir.to_vector());
    }
}

pub type BlipIndex = usize;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum LevelStatus {
    Running,
    Completed,
    Failed,
}

pub type Activation = Option<BlipIndex>;

struct BlocksState {
    wind_out: Vec<DirMap3<bool>>,
    activation: Vec<Activation>,
}

impl BlocksState {
    fn new_initial(machine: &Machine) -> Self {
        // We assume that the machine's blocks are contiguous in memory, so that
        // we can store block state as a Vec, instead of wasting memory or
        // cycles on VecOption while executing.
        assert!(machine.is_contiguous());

        State {
            wind_out: vec![DirMap3::default(); machine.num_blocks()],
            activation: vec![Activation::default(); machine.num_blocks()],
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
    next_blocks: BlocksState,

    next_blip_count: Vec<usize>,
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
            next_blocks: BlocksState::new_initial(&machine),
            next_blip_count: vec![0; machine.num_blocks()],
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
        // 1) Spawn and move wind.
        mem::swap(&mut self.blocks.wind_out, &mut self.next_blocks.wind_out);

        for block_index in 0..self.machine.num_blocks() {
            self.next_blocks.wind_out[block_index] = spawn_or_advect_wind(
                block_index,
                &self.machine,
                &self.neighbor_map,
                &self.blocks.wind_out,
                &self.blocks.activation,
            );
        }

        // 2) Remove dead blips.
        self.blips.retain(|blip| !blip.state.is_dead());

        // 3) Perform blip movement as it was defined in the previous update,
        //    then determine new blip movement direction.
        for (_, blip) in self.blips.iter_mut() {
            // At this point, there are only non-dead blips. Blips that spawned
            // in the previous update are now fully grown.
            blip.status = BlipStatus::Existing;

            if let Some(move_dir) = blip.move_dir {
                blip.pos = blip.pos + move_dir.to_vector();
            }

            blip.move_dir = blip_move_dir(
                blip,
                &self.machine,
                &self.neighbor_map,
                &self.next_blocks.wind_out,
            );
        }

        // 4) At each block, count blips that will be there next tick, after
        //    movement.
        for count in self.next_blip_count.iter_mut() {
            *count = 0;
        }

        for (_, blip) in self.blips.iter() {
            debug_assert!(!blip.is_spawning());

            if let Some((next_block_index, next_block)) = self.machine.get(blip.next_pos()) {
                self.next_blip_count[next_block_index] += 1;
            }
        }

        // 5) Run effects of blocks that are activated in this tick.
        mem::swap(
            &mut self.blocks.activation,
            &mut self.blocks.next_activation,
        );

        for (block_index, (block_pos, placed_block)) in self.machine.blocks.data.iter_mut() {
            if let Some(kind) = self_activate_block(
                block_index,
                &mut placed_block.block,
                &self.neighbor_map,
                &self.next_blip_count,
            ) {
                self.blocks.activation[block_index] =
                    cmp::max(self.blocks.activation[block_index], Some(kind));
            }

            if let Some(blip_kind) = self.blocks.activation[block_index] {
                run_activated_block(*block_pos, &placed_block.block, blip_kind, &mut self.blips);
            }
        }

        // The block activations may have spawned new blips. These need to be
        // counted, lest we lose control over our population.
        for (_, blip) in self.blips.iter() {
            if blip.is_spawning() {
                if let Some((next_block_index, next_block)) = self.machine.get(blip.next_pos()) {
                    self.next_blip_count[next_block_index] += 1;
                }
            }
        }

        // 7) Determine next activations based on blips and handle blip-blip
        //    collisions.
        for activation in self.next_blocks.activation.iter_mut() {
            *activation = None;
        }

        for (_, blip) in self.blips.iter_mut() {
            if let Some((next_block_index, next_block)) = self.machine.get(blip.next_pos()) {
                if self.next_block_count[next_block_index] > 0 {
                    // We ran into another blip.
                    blip.status = blip.status.kill();
                }

                if blip.move_dir.is_some() || blip.is_spawning() {
                    if next_block.is_activatable(blip.kind) {
                        // This block's effect will run in the next tick.
                        self.next_blocks.activation[next_block_index] = cmp::max(
                            self.next_blocks.activation[next_block_index],
                            Some(blip.kind),
                        );
                    }

                    if next_block.is_blip_killer() {
                        blip.status = blip.status.kill();
                    }
                }
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
        Block::WindSource => DirMap3::from_fn(|_| true),
        Block::BlipWindSource { button_dir } => {
            if prev_activation[block_index].is_some() {
                DirMap3::from_fn(|dir| dir != *button_dir)
            } else {
                DirMap3::from_fn(false)
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
                DirMap3::from_fn(false)
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
    let block = &machine.get(blip.pos)?.block;

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
                // TODO: I don't think this can actually happen with our Block cases.
                None
            }
        }
        _ => None,
    }
}

fn self_activate_block(
    block_index: &Point3,
    block: &mut Block,
    neighbor_map: &NeighborMap,
    next_blip_count: &[usize],
) -> Option<BlipKind> {
    match block {
        Block::BlipSpawn {
            out_dir,
            kind,
            ref mut num_spawns,
        } => {
            if let Some(neighbor_index) = neighbor_map.lookup(block_index, out_dir) {
                // The blip spawn only acts if there is no blip at the output position.
                if next_blip_count[neighbor_index] == 0 && num_spawns.map_or(true, |n| n > 0) {
                    *num_spawns = num_spawns.map_or(None, |n| Some(n - 1));
                    return Some(kind);
                }
            }
        }
    }

    None
}

fn run_activated_block(
    block_pos: &Point3,
    block: &Block,
    blip_kind: BlipKind,
    blips: &mut VecOption<Blip>,
) {
    match block {
        Block::BlipSpawn { out_dir, kind, .. } => {
            blips.add(Blip {
                kind: blip_kind,
                pos: block_pos,
                move_dir: Some(out_dir),
                status: BlipStatus::Spawning(BlipSpawnMode::Quick),
            });
        }
        Block::BlipDuplicator { out_dirs, .. } => {
            for &dir in &[out_dir.0, out_dirs.1] {
                blips.add(Blip {
                    kind: blip_kind,
                    pos: block_pos,
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
