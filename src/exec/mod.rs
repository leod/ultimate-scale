pub mod anim;
pub mod level;
pub mod neighbors;
pub mod play;
#[cfg(test)]
mod tests;
pub mod view;

use std::cmp;
use std::collections::HashSet;
use std::mem;

use coarse_prof::profile;
use log::info;
use rand::Rng;

use crate::machine::grid::{Axis3, Dir3, DirMap3, Point3, Vector3};
use crate::machine::{BlipKind, Block, BlockIndex, Machine, PlacedBlock, TickNum};
use crate::util::vec_option::VecOption;

use neighbors::NeighborMap;

pub use level::{LevelProgress, LevelStatus};
pub use play::TickTime;
pub use view::ExecView;

/// Ways that blips can enter live.
#[derive(PartialEq, Eq, Copy, Clone, Debug, Hash)]
pub enum BlipSpawnMode {
    //Ease,
    Quick,
    Bridge,
}

/// Ways that blips can leave live.
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Hash)]
pub enum BlipDieMode {
    PopEarly,
    PopMiddle,
    PressButton,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug, Hash)]
pub enum BlipStatus {
    Spawning(BlipSpawnMode),
    Existing,
    LiveToDie(BlipSpawnMode, BlipDieMode),
    Dying(BlipDieMode),
}

impl BlipStatus {
    fn is_spawning(self) -> bool {
        match self {
            BlipStatus::Spawning(_) => true,
            BlipStatus::LiveToDie(_, _) => true,
            _ => false,
        }
    }

    pub fn is_existing(self) -> bool {
        match self {
            BlipStatus::Existing => true,
            _ => false,
        }
    }

    fn is_dead(self) -> bool {
        match self {
            BlipStatus::Dying(_) => true,
            BlipStatus::LiveToDie(_, _) => true,
            _ => false,
        }
    }

    fn is_pressing_button(self) -> bool {
        match self {
            BlipStatus::Dying(BlipDieMode::PressButton) => true,
            BlipStatus::LiveToDie(_, BlipDieMode::PressButton) => true,
            _ => false,
        }
    }

    fn is_bridge_spawning(self) -> bool {
        match self {
            BlipStatus::Spawning(BlipSpawnMode::Bridge) => true,
            BlipStatus::LiveToDie(BlipSpawnMode::Bridge, _) => true,
            _ => false,
        }
    }

    fn kill(&mut self, new_die_mode: BlipDieMode) {
        *self = match *self {
            BlipStatus::Spawning(spawn_mode) => BlipStatus::LiveToDie(spawn_mode, new_die_mode),
            BlipStatus::Existing => BlipStatus::Dying(new_die_mode),
            BlipStatus::LiveToDie(spawn_mode, die_mode) => {
                BlipStatus::LiveToDie(spawn_mode, die_mode.min(new_die_mode))
            }
            BlipStatus::Dying(die_mode) => BlipStatus::Dying(die_mode.min(new_die_mode)),
        }
    }

    fn die_mode(self) -> Option<BlipDieMode> {
        match self {
            BlipStatus::LiveToDie(_, die_mode) => Some(die_mode),
            BlipStatus::Dying(die_mode) => Some(die_mode),
            _ => None,
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

pub type BlipIndex = usize;

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

    prev_activation: Vec<Activation>,

    next_blip_count: Vec<usize>,
}

impl Exec {
    pub fn new<R: Rng + ?Sized>(mut machine: Machine, rng: &mut R) -> Exec {
        // Make the machine's blocks contiguous in memory.
        machine.gc();

        initialize_air_blocks(&mut machine);

        let neighbor_map = NeighborMap::new_from_machine(&machine);
        let level_progress = machine.level.as_ref().map(|level| {
            let inputs_outputs = level.spec.gen_inputs_outputs(rng);
            LevelProgress::new(Some(&machine), inputs_outputs)
        });
        let next_level_progress = level_progress.clone();
        let blocks = BlocksState::new_initial(&machine);
        let next_blocks = BlocksState::new_initial(&machine);
        let prev_activation = vec![None; machine.num_blocks()];
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
            prev_activation,
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

    pub fn prev_activation(&self) -> &[Activation] {
        &self.prev_activation
    }

    pub fn update(&mut self) {
        // 1) Advance state.
        self.level_progress = self.next_level_progress.clone();

        // Next wind_out will be written from scratch in step 2.
        mem::swap(&mut self.blocks.wind_out, &mut self.next_blocks.wind_out);

        // Pass along activation triple-buffer
        mem::swap(&mut self.prev_activation, &mut self.next_blocks.activation);
        mem::swap(&mut self.prev_activation, &mut self.blocks.activation);
        for activation in self.next_blocks.activation.iter_mut() {
            *activation = None;
        }

        // 2) Spawn and move wind.
        {
            profile!("wind");

            for block_index in 0..self.machine.num_blocks() {
                self.next_blocks.wind_out[block_index] = spawn_or_advect_wind(
                    block_index,
                    &self.machine,
                    &self.neighbor_map,
                    &self.blocks.wind_out,
                    &self.prev_activation,
                    &self.blocks.activation,
                );
            }
        }

        // 3) Remove dead blips.
        {
            profile!("clean_up");

            self.blips.retain(|blip| !blip.status.is_dead());
        }

        // 4) Perform blip movement as it was defined in the previous update,
        //    then determine new blip movement direction.
        {
            profile!("move");

            for (_, blip) in self.blips.iter_mut() {
                // At this point, there are only non-dead blips. Blips that spawned
                // in the previous update are now fully grown.
                blip.status = BlipStatus::Existing;

                if let Some(move_dir) = blip.move_dir {
                    blip.pos += move_dir.to_vector();
                    blip.orient = move_dir;
                }

                blip.move_dir = blip_move_dir(
                    blip,
                    &self.machine,
                    &self.neighbor_map,
                    &self.blocks.wind_out,
                    &self.next_blocks.wind_out,
                    &self.blocks.activation,
                );
            }
        }

        // 5) At each block, count blips that will be there next tick, after
        //    movement.
        {
            profile!("count");

            for count in self.next_blip_count.iter_mut() {
                *count = 0;
            }

            for (_, blip) in self.blips.iter() {
                debug_assert!(!blip.status.is_spawning());

                if let Some((next_block_index, next_block)) =
                    self.machine.get_with_index(&blip.next_pos())
                {
                    // Don't count blips that will kill get killed by their
                    // new block in step 7.
                    let will_die_anyway = next_block
                        .block
                        .is_blip_killer(blip.move_dir.map(|d| d.invert()))
                        .is_some();

                    if !will_die_anyway {
                        self.next_blip_count[next_block_index] += 1;
                    }
                }
            }
        }

        // 6) Run effects of blocks that are activated in this tick.
        {
            profile!("effects");

            for block_index in self.machine.blocks.data.keys() {
                if let Some(kind) = self_activate_block(
                    block_index,
                    &self.machine.blocks.data,
                    &mut self.level_progress,
                    &self.neighbor_map,
                    &self.next_blip_count,
                ) {
                    self.blocks.activation[block_index] =
                        cmp::max(self.blocks.activation[block_index], Some(kind));
                }
            }

            for (block_index, (block_pos, placed_block)) in self.machine.blocks.data.iter_mut() {
                if let Some(blip_kind) = self.prev_activation[block_index] {
                    run_prev_activated_block(
                        block_pos,
                        &placed_block.block,
                        blip_kind,
                        &mut self.blips,
                    );
                }

                if let Some(blip_kind) = self.blocks.activation[block_index] {
                    run_activated_block(
                        block_index,
                        block_pos,
                        &mut placed_block.block,
                        blip_kind,
                        &mut self.blips,
                        &self.neighbor_map,
                        &self.next_blip_count,
                    );
                }
            }

            // The block activations may have spawned new blips. These need to be
            // counted, lest we lose control over our population.
            for (_, blip) in self.blips.iter() {
                if blip.status.is_spawning() {
                    if let Some((next_block_index, next_block)) =
                        self.machine.get_with_index(&blip.next_pos())
                    {
                        // Don't count blips that will kill get killed by their
                        // new block in step 7.
                        let will_die_anyway = next_block
                            .block
                            .is_blip_killer(blip.move_dir.map(|d| d.invert()))
                            .is_some();

                        if !will_die_anyway {
                            self.next_blip_count[next_block_index] += 1;
                        }
                    }
                }
            }
        }

        // 7) Determine next activations based on blips and update blip status
        //    based on next position.
        //    Any code that modifies a blip's state to be dying is here.
        {
            profile!("activate");

            for (_, blip) in self.blips.iter_mut() {
                if let Some((next_block_index, next_block)) =
                    self.machine.get_with_index(&blip.next_pos())
                {
                    if self.next_blip_count[next_block_index] > 1 {
                        // We ran into another blip.
                        blip.status.kill(BlipDieMode::PopMiddle);
                    }

                    let activation_borrow = &self.blocks.activation[next_block_index];
                    let is_move_blocked = blip.move_dir.map_or(false, |move_dir| {
                        !next_block
                            .block
                            .has_move_hole(move_dir.invert(), activation_borrow.is_some())
                    });

                    if is_move_blocked && !next_block.block.is_pipe() {
                        // The blip is moving into a block that does not have an
                        // opening in this direction.
                        blip.status.kill(BlipDieMode::PopEarly);
                    } else {
                        let inverse_dir = blip.move_dir.map(|d| d.invert());
                        let activate = !blip.status.is_dead()
                            && next_block.block.is_activatable(blip.kind, inverse_dir);

                        if activate {
                            // This block's effect will run in the next tick.
                            self.next_blocks.activation[next_block_index] = cmp::max(
                                self.next_blocks.activation[next_block_index],
                                Some(blip.kind),
                            );
                        }

                        if let Some(die_mode) = next_block.block.is_blip_killer(inverse_dir) {
                            blip.status.kill(die_mode);
                        }
                    }
                } else {
                    // Blip is out of bounds or not on a block.
                    blip.status.kill(BlipDieMode::PopEarly);
                }
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

fn initialize_air_blocks(machine: &mut Machine) {
    let air_blocks: HashSet<_> = {
        let machine: &Machine = machine;

        machine
            .iter_blocks()
            .flat_map(|(_, (pos, block))| {
                Dir3::ALL.iter().flat_map(move |dir| {
                    let mut result = Vec::new();

                    let build_air = (block.block.has_wind_hole_out(*dir, false)
                        && block.block.has_move_hole(*dir, false))
                        || block.block.has_blip_spawn(*dir);

                    if build_air {
                        let mut iter_pos = pos + dir.to_vector();

                        while machine.is_valid_pos(&iter_pos) && !machine.is_block_at(&iter_pos) {
                            result.push(iter_pos);
                            iter_pos.z -= 1;
                        }
                    }

                    result
                })
            })
            .collect()
    };

    info!("Adding {} air blocks to machine", air_blocks.len());

    for pos in air_blocks {
        assert!(!machine.is_block_at(&pos));
        machine.set(&pos, Some(PlacedBlock { block: Block::Air }));
    }
}

fn advect_wind(
    block_index: BlockIndex,
    machine: &Machine,
    neighbor_map: &NeighborMap,
    wind_out: &[DirMap3<bool>],
    activation: &[Activation],
) -> DirMap3<bool> {
    let block = machine.block_at_index(block_index);

    // Check if we got any wind in flow from our neighbors in the
    // old state
    let block_wind_in = neighbor_map[block_index].map(|dir, neighbor_index| {
        neighbor_index.map_or(false, |neighbor_index| {
            block.has_wind_hole_in(dir, activation[block_index].is_some())
                && wind_out[neighbor_index][dir.invert()]
        })
    });

    if block_wind_in.values().any(|flow| *flow) {
        // Forward in flow to our outgoing wind hole directions
        neighbor_map[block_index].map(|dir, neighbor_index| {
            let hole_out = block.has_wind_hole_out(dir, activation[block_index].is_some());
            let no_wind_in = neighbor_index.map_or(true, |_| !block_wind_in[dir]);
            let neighbor_hole_in = neighbor_index.map_or(true, |neighbor_index| {
                machine
                    .block_at_index(neighbor_index)
                    .has_wind_hole_in(dir.invert(), activation[neighbor_index].is_some())
            });

            hole_out && no_wind_in && neighbor_hole_in
        })
    } else {
        DirMap3::from_fn(|_| false)
    }
}

fn spawn_or_advect_wind(
    block_index: BlockIndex,
    machine: &Machine,
    neighbor_map: &NeighborMap,
    wind_out: &[DirMap3<bool>],
    prev_activation: &[Activation],
    activation: &[Activation],
) -> DirMap3<bool> {
    let block = machine.block_at_index(block_index);

    match block {
        Block::WindSource => DirMap3::from_fn(|_| true),
        Block::BlipWindSource { .. } => {
            if activation[block_index].is_some() {
                DirMap3::from_fn(|dir| block.has_wind_source(dir))
            } else {
                DirMap3::from_fn(|_| false)
            }
        }
        Block::Input { out_dir, .. } => DirMap3::from_fn(|dir| dir == *out_dir),
        Block::DetectorWindSource { .. } => {
            let pipe = advect_wind(block_index, machine, neighbor_map, wind_out, activation);

            if activation[block_index].is_some() {
                DirMap3::from_fn(|dir| block.has_wind_source(dir) || pipe[dir])
            } else {
                pipe
            }
        }
        Block::Delay { flow_dir } => {
            if prev_activation[block_index].is_some() {
                DirMap3::from_fn(|dir| dir == *flow_dir)
            } else {
                DirMap3::from_fn(|_| false)
            }
        }
        _ => advect_wind(block_index, machine, neighbor_map, wind_out, activation),
    }
}

fn blip_move_dir(
    blip: &Blip,
    machine: &Machine,
    neighbor_map: &NeighborMap,
    wind_out: &[DirMap3<bool>],
    next_wind_out: &[DirMap3<bool>],
    activation: &[Activation],
) -> Option<Dir3> {
    let (block_index, placed_block) = machine.get_with_index(&blip.pos)?;
    let block = &placed_block.block;
    let is_active = activation[block_index].is_some();

    let block_move_out = neighbor_map[block_index].map(|dir, neighbor_index| {
        neighbor_index.map_or(false, |neighbor_index| {
            let neighbor_block = machine.block_at_index(neighbor_index);

            let can_move_out =
                next_wind_out[block_index][dir] && block.has_move_hole(dir, is_active);

            let can_move_in =
                dir == Dir3::Z_NEG || neighbor_block.has_move_hole(dir.invert(), is_active);
            //&& neighbor_block.has_wind_hole_in(dir.invert(), is_active)

            can_move_out && can_move_in
        })
    });

    let block_wind_in = neighbor_map[block_index].map(|dir, neighbor_index| {
        neighbor_index.map_or(false, |neighbor_index| {
            block.has_wind_hole_in(dir, is_active) && wind_out[neighbor_index][dir.invert()]
        })
    });

    let can_move = |dir: Dir3| block_move_out[dir] && !block_wind_in[dir];

    let num_can_move: usize = Dir3::ALL.iter().filter(|dir| can_move(**dir)).count();

    let must_fall = *block == Block::Air
        || ((*block == Block::PipeButton { axis: Axis3::X }
            || *block == Block::PipeButton { axis: Axis3::Y })
            && is_active);

    let turn_to_side =
        |dir: Dir3| dir != blip.orient && can_move(dir) && block_wind_in[dir.invert()];
    let num_turn_to_side = Dir3::ALL.iter().filter(|dir| turn_to_side(**dir)).count();

    if must_fall {
        // The only way is DOWN!
        Some(Dir3::Z_NEG)
    } else if num_turn_to_side == 1 {
        Dir3::ALL.iter().cloned().find(|dir| turn_to_side(*dir))
    } else if can_move(blip.orient) {
        Some(blip.orient)
    } else if num_can_move == 1 {
        Dir3::ALL.iter().cloned().find(|dir| can_move(*dir))
    } else {
        None
    }
}

fn self_activate_block(
    block_index: BlockIndex,
    blocks: &VecOption<(Point3, PlacedBlock)>,
    level_progress: &mut Option<LevelProgress>,
    neighbor_map: &NeighborMap,
    next_blip_count: &[usize],
) -> Option<BlipKind> {
    match blocks[block_index].1.block.clone() {
        Block::BlipSpawn {
            out_dir,
            kind,
            num_spawns,
        } => {
            if let Some(neighbor_index) = neighbor_map[block_index][out_dir] {
                // The blip spawn acts only if there is no blip at the output position.
                let is_safe = next_blip_count[neighbor_index] == 0
                    || blocks[neighbor_index]
                        .1
                        .block
                        .is_blip_killer(Some(out_dir))
                        .is_some();
                if is_safe && num_spawns.map_or(true, |n| n > 0) {
                    return Some(kind);
                }
            }
        }
        Block::Input { out_dir, index } => {
            if let Some(neighbor_index) = neighbor_map[block_index][out_dir] {
                // The input acts only if there is no blip at the output position.
                if next_blip_count[neighbor_index] == 0 {
                    return level_progress.as_mut().and_then(|p| p.feed_input(index));
                }
            }
        }
        _ => (),
    }

    None
}

fn run_prev_activated_block(
    block_pos: &Point3,
    block: &Block,
    blip_kind: BlipKind,
    blips: &mut VecOption<Blip>,
) {
    match block {
        Block::Delay { flow_dir } => {
            blips.add(Blip::new(
                blip_kind,
                *block_pos,
                *flow_dir,
                Some(*flow_dir),
                BlipSpawnMode::Quick,
            ));
        }
        _ => (),
    }
}

fn run_activated_block(
    block_index: BlockIndex,
    block_pos: &Point3,
    block: &mut Block,
    blip_kind: BlipKind,
    blips: &mut VecOption<Blip>,
    neighbor_map: &NeighborMap,
    next_blip_count: &[usize],
) {
    match block {
        Block::BlipSpawn {
            out_dir,
            kind,
            num_spawns,
            ..
        } => {
            *num_spawns = num_spawns.map_or(None, |n| Some(n - 1));
            blips.add(Blip::new(
                *kind,
                *block_pos,
                *out_dir,
                Some(*out_dir),
                BlipSpawnMode::Bridge,
            ));
        }
        Block::BlipDuplicator { out_dirs, .. } => {
            for &out_dir in &[out_dirs.0, out_dirs.1] {
                let neighbor_index = neighbor_map[block_index][out_dir];
                let is_free = neighbor_index
                    .map_or(true, |neighbor_index| next_blip_count[neighbor_index] == 0);

                if is_free {
                    blips.add(Blip::new(
                        blip_kind,
                        *block_pos,
                        out_dir,
                        Some(out_dir),
                        BlipSpawnMode::Bridge,
                    ));
                }
            }
        }
        Block::Input { out_dir, .. } => {
            blips.add(Blip::new(
                blip_kind,
                *block_pos,
                *out_dir,
                Some(*out_dir),
                BlipSpawnMode::Bridge,
            ));
        }
        Block::DetectorBlipDuplicator { out_dir, .. } => {
            blips.add(Blip::new(
                blip_kind,
                *block_pos,
                *out_dir,
                Some(*out_dir),
                BlipSpawnMode::Quick,
            ));
        }
        Block::BlipDeleter { out_dirs, .. } => {
            for &out_dir in &[out_dirs.0, out_dirs.1] {
                let neighbor_index = neighbor_map[block_index][out_dir];
                let is_free = neighbor_index
                    .map_or(true, |neighbor_index| next_blip_count[neighbor_index] == 0);

                if !is_free {
                    blips.add(Blip::new(
                        blip_kind,
                        *block_pos,
                        out_dir,
                        Some(out_dir),
                        BlipSpawnMode::Bridge,
                    ));
                }
            }
        }
        _ => (),
    }
}
