pub mod anim;
pub mod level;
pub mod neighbors;
pub mod play;
#[cfg(test)]
mod tests;
pub mod view;

use std::cmp;
use std::iter;
use std::mem;

use rand::Rng;

use crate::machine::grid::{Dir3, DirMap3, Point3, Vector3};
use crate::machine::{BlipKind, Block, BlockIndex, Machine, TickNum};
use crate::util::vec_option::VecOption;

use neighbors::NeighborMap;

pub use level::{LevelProgress, LevelStatus};
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
        match self {
            BlipStatus::Spawning(_) => true,
            BlipStatus::LiveToDie => true,
            _ => false,
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

    /// The blip's orientation. Determines movement preferences.
    pub orient: Dir3,

    /// The direction in which the blip moved last tick, if any.
    pub move_dir: Option<Dir3>,

    /// Status. Used mostly for visual purposes. Blips marked as Dying will
    /// be removed at the start of the next tick.
    pub status: BlipStatus,
}

impl Blip {
    fn new(
        kind: BlipKind,
        pos: Point3,
        orient: Dir3,
        move_dir: Option<Dir3>,
        spawn_mode: BlipSpawnMode,
    ) -> Self {
        Blip {
            kind,
            pos,
            orient,
            move_dir,
            status: BlipStatus::Spawning(spawn_mode),
        }
    }

    fn next_pos(&self) -> Point3 {
        self.pos
            + self
                .move_dir
                .map_or(Vector3::zeros(), |dir| dir.to_vector())
    }

    fn next_orient(&self) -> Dir3 {
        self.move_dir.unwrap_or(self.orient)
    }

    fn is_turning(&self) -> bool {
        self.move_dir.map_or(false, |dir| dir != self.orient)
    }
}

pub type Activation = Option<BlipKind>;

pub struct BlocksState {
    pub wind_out: Vec<DirMap3<bool>>,
    pub activation: Vec<Activation>,
}

impl BlocksState {
    fn new_initial(machine: &Machine) -> Self {
        // We assume that the machine's blocks are contiguous in memory, so that
        // we can store block state as a Vec, instead of wasting memory or
        // cycles on VecOption while executing.
        assert!(machine.is_contiguous());

        Self {
            wind_out: vec![DirMap3::default(); machine.num_blocks()],
            activation: vec![Activation::default(); machine.num_blocks()],
        }
    }
}

pub struct Exec {
    cur_tick: TickNum,

    machine: Machine,
    neighbor_map: NeighborMap,

    level_progress: Option<LevelProgress>,
    next_level_progress: Option<LevelProgress>,

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
        let level_progress = machine.level.as_ref().map(|level| {
            let inputs_outputs = level.spec.gen_inputs_outputs(rng);
            LevelProgress::new(Some(&machine), inputs_outputs)
        });
        let next_level_progress = level_progress.clone();
        let blocks = BlocksState::new_initial(&machine);
        let next_blocks = BlocksState::new_initial(&machine);
        let next_blip_count = vec![0; machine.num_blocks()];

        Exec {
            cur_tick: 0,
            machine,
            neighbor_map,
            level_progress,
            next_level_progress,
            blips: VecOption::new(),
            blocks,
            next_blocks,
            next_blip_count,
        }
    }

    pub fn machine(&self) -> &Machine {
        &self.machine
    }

    pub fn neighbor_map(&self) -> &NeighborMap {
        &self.neighbor_map
    }

    pub fn level_progress(&self) -> Option<&LevelProgress> {
        self.level_progress.as_ref()
    }

    pub fn next_level_progress(&self) -> Option<&LevelProgress> {
        self.next_level_progress.as_ref()
    }

    pub fn blips(&self) -> &VecOption<Blip> {
        &self.blips
    }

    pub fn blocks(&self) -> &BlocksState {
        &self.blocks
    }

    pub fn next_blocks(&self) -> &BlocksState {
        &self.next_blocks
    }

    pub fn update(&mut self) {
        // 1) Update level state.
        self.level_progress = self.next_level_progress.clone();

        // 2) Spawn and move wind.
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

        // 3) Remove dead blips.
        self.blips.retain(|blip| !blip.status.is_dead());

        // 4) Perform blip movement as it was defined in the previous update,
        //    then determine new blip movement direction.
        for (_, blip) in self.blips.iter_mut() {
            // At this point, there are only non-dead blips. Blips that spawned
            // in the previous update are now fully grown.
            blip.status = BlipStatus::Existing;

            if let Some(move_dir) = blip.move_dir {
                blip.pos = blip.pos + move_dir.to_vector();
                blip.orient = move_dir;
            }

            blip.move_dir = blip_move_dir(
                blip,
                &self.machine,
                &self.neighbor_map,
                &self.blocks.wind_out,
                &self.next_blocks.wind_out,
            );
        }

        // 5) At each block, count blips that will be there next tick, after
        //    movement.
        for count in self.next_blip_count.iter_mut() {
            *count = 0;
        }

        for (_, blip) in self.blips.iter() {
            debug_assert!(!blip.status.is_spawning());

            if let Some(next_block_index) = self.machine.get_index(&blip.next_pos()) {
                self.next_blip_count[next_block_index] += 1;
            }
        }

        // 6) Run effects of blocks that are activated in this tick.
        mem::swap(
            &mut self.blocks.activation,
            &mut self.next_blocks.activation,
        );

        for (block_index, (block_pos, placed_block)) in self.machine.blocks.data.iter_mut() {
            if let Some(kind) = self_activate_block(
                block_index,
                &mut placed_block.block,
                &mut self.level_progress,
                &self.neighbor_map,
                &self.next_blip_count,
            ) {
                self.blocks.activation[block_index] =
                    cmp::max(self.blocks.activation[block_index], Some(kind));
            }

            if let Some(blip_kind) = self.blocks.activation[block_index] {
                run_activated_block(block_pos, &placed_block.block, blip_kind, &mut self.blips);
            }
        }

        // The block activations may have spawned new blips. These need to be
        // counted, lest we lose control over our population.
        for (_, blip) in self.blips.iter() {
            if blip.status.is_spawning() {
                if let Some(next_block_index) = self.machine.get_index(&blip.next_pos()) {
                    self.next_blip_count[next_block_index] += 1;
                }
            }
        }

        // 7) Determine next activations based on blips and update blip status
        //    based on next position.
        for activation in self.next_blocks.activation.iter_mut() {
            *activation = None;
        }

        for (_, blip) in self.blips.iter_mut() {
            let mut kill = false;

            if let Some((next_block_index, next_block)) =
                self.machine.get_with_index(&blip.next_pos())
            {
                if self.next_blip_count[next_block_index] > 1 {
                    // We ran into another blip.
                    kill = true;
                }

                if blip.move_dir.is_some() || blip.status.is_spawning() {
                    if next_block.block.is_activatable(blip.kind) {
                        // This block's effect will run in the next tick.
                        self.next_blocks.activation[next_block_index] = cmp::max(
                            self.next_blocks.activation[next_block_index],
                            Some(blip.kind),
                        );
                    }

                    kill = kill || next_block.block.is_blip_killer();
                }
            } else {
                kill = true;
            }

            if kill {
                blip.status = blip.status.kill();
            }
        }

        // 8) Determine the next level progress based on block activations.
        //    This allows us to see if the level will be completed or failed
        //    next tick, so we can stop the playback early, allowing the player
        //    to see which blips exactly caused completion or failure.
        self.next_level_progress = self.level_progress.as_ref().map(|progress| {
            let mut next_progress = progress.clone();
            next_progress.update_outputs(&self.next_blocks.activation);
            next_progress
        });

        self.cur_tick += 1;
    }
}

fn spawn_or_advect_wind(
    block_index: BlockIndex,
    machine: &Machine,
    neighbor_map: &NeighborMap,
    wind_out: &[DirMap3<bool>],
    prev_activation: &[Activation],
) -> DirMap3<bool> {
    let block = machine.block_at_index(block_index);

    match block {
        Block::WindSource => DirMap3::from_fn(|_| true),
        Block::BlipWindSource { button_dir } => {
            if prev_activation[block_index].is_some() {
                DirMap3::from_fn(|dir| dir != *button_dir)
            } else {
                DirMap3::from_fn(|_| false)
            }
        }
        Block::Input { out_dir, .. } => DirMap3::from_fn(|dir| dir == *out_dir),
        _ => {
            // Check if we got any wind in flow from our neighbors in the
            // old state
            let block_wind_in = neighbor_map[block_index].map(|dir, neighbor_index| {
                neighbor_index.map_or(false, |neighbor_index| {
                    block.has_wind_hole_in(dir) && wind_out[neighbor_index][dir.invert()]
                })
            });

            if block_wind_in.values().any(|flow| *flow) {
                // Forward in flow to our outgoing wind hole directions
                neighbor_map[block_index].map(|dir, neighbor_index| {
                    neighbor_index.map_or(false, |_| {
                        block.has_wind_hole_out(dir) && !block_wind_in[dir]
                    })
                })
            } else {
                DirMap3::from_fn(|_| false)
            }
        }
    }
}

fn find_dir_ccw_xy(initial_dir: Dir3, f: impl Fn(Dir3) -> bool) -> Option<Dir3> {
    iter::successors(Some(initial_dir), |dir| Some(dir.rotated_ccw_xy()))
        .take(4)
        .find(|dir| f(*dir))
}

fn blip_move_dir(
    blip: &Blip,
    machine: &Machine,
    neighbor_map: &NeighborMap,
    wind_out: &[DirMap3<bool>],
    next_wind_out: &[DirMap3<bool>],
) -> Option<Dir3> {
    let (block_index, placed_block) = machine.get_with_index(&blip.pos)?;
    let block = &placed_block.block;

    let block_move_out = neighbor_map[block_index].map(|dir, neighbor_index| {
        neighbor_index.map_or(false, |neighbor_index| {
            let neighbor_block = machine.block_at_index(neighbor_index);

            next_wind_out[block_index][dir]
                && block.has_move_hole(dir)
                && neighbor_block.has_move_hole(dir.invert())
                && neighbor_block.has_wind_hole_in(dir.invert())
        })
    });

    let block_wind_in = neighbor_map[block_index].map(|dir, neighbor_index| {
        neighbor_index.map_or(false, |neighbor_index| {
            block.has_wind_hole_in(dir) && wind_out[neighbor_index][dir.invert()]
        })
    });

    let num_move_out: usize = block_move_out.values().map(|flow| *flow as usize).sum();

    let can_move = |dir: Dir3| block_move_out[dir] && !block_wind_in[dir];

    if num_move_out == 1 {
        Dir3::ALL.iter().cloned().find(|dir| can_move(*dir))
    } else {
        find_dir_ccw_xy(blip.orient, can_move)
    }
}

fn self_activate_block(
    block_index: BlockIndex,
    block: &mut Block,
    level_progress: &mut Option<LevelProgress>,
    neighbor_map: &NeighborMap,
    next_blip_count: &[usize],
) -> Option<BlipKind> {
    match block {
        Block::BlipSpawn {
            out_dir,
            kind,
            ref mut num_spawns,
        } => {
            if let Some(neighbor_index) = neighbor_map[block_index][*out_dir] {
                // The blip spawn acts only if there is no blip at the output position.
                if next_blip_count[neighbor_index] == 0 && num_spawns.map_or(true, |n| n > 0) {
                    *num_spawns = num_spawns.map_or(None, |n| Some(n - 1));
                    return Some(*kind);
                }
            }
        }
        Block::Input { out_dir, index } => {
            if let Some(neighbor_index) = neighbor_map[block_index][*out_dir] {
                // The input acts only if there is no blip at the output position.
                if next_blip_count[neighbor_index] == 0 {
                    return level_progress.as_mut().and_then(|p| p.feed_input(*index));
                }
            }
        }
        _ => (),
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
            blips.add(Blip::new(
                *kind,
                *block_pos,
                *out_dir,
                Some(*out_dir),
                BlipSpawnMode::Quick,
            ));
        }
        Block::BlipDuplicator { out_dirs, .. } => {
            for &out_dir in &[out_dirs.0, out_dirs.1] {
                blips.add(Blip::new(
                    blip_kind,
                    *block_pos,
                    out_dir,
                    Some(out_dir),
                    BlipSpawnMode::Quick,
                ));
            }
        }
        Block::Input { out_dir, .. } => {
            blips.add(Blip::new(
                blip_kind,
                *block_pos,
                *out_dir,
                Some(*out_dir),
                BlipSpawnMode::Quick,
            ));
        }
        _ => (),
    }
}
