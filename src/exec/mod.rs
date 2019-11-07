pub mod anim;
pub mod play;
pub mod view;

use log::debug;

use crate::machine::grid::{Dir3, Grid3, Point3};
use crate::machine::{BlipKind, Block, BlockIndex, Machine, PlacedBlock, TickNum};
use crate::util::vec_option::VecOption;

pub use play::TickTime;
pub use view::ExecView;

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct BlipMovement {
    pub dir: Dir3,
    pub progress: usize,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum BlipStatus {
    Spawning,
    Existing,
    Dying,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct Blip {
    pub kind: BlipKind,
    pub pos: Point3,

    /// The direction in which the blip moved last tick, if any.
    pub old_move_dir: Option<Dir3>,

    /// Has this blip moved in the previous frame? If true, effects for
    /// entering block will be applied in the next tick
    pub moved: bool,

    /// Status. Used mostly for visual purposes. Blips marked as Dying will
    /// be removed at the start of the next tick.
    pub status: BlipStatus,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Default)]
pub struct WindState {
    pub wind_in: [bool; Dir3::NUM_INDICES],
}

impl WindState {
    pub fn wind_in(self, dir: Dir3) -> bool {
        self.wind_in[dir.to_index()]
    }
}

pub type BlipIndex = usize;

#[derive(PartialEq, Eq, Clone, Copy, Debug, Default)]
pub struct BlipState {
    pub blip_index: Option<BlipIndex>,
}

pub struct Exec {
    cur_tick: TickNum,

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
            cur_tick: 0,
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

    pub fn old_wind_state(&self) -> &[WindState] {
        &self.old_wind_state
    }

    pub fn blips(&self) -> &VecOption<Blip> {
        &self.blips
    }

    pub fn update(&mut self) {
        self.check_consistency();

        self.old_wind_state[..].clone_from_slice(&self.wind_state);
        self.old_blip_state[..].clone_from_slice(&self.blip_state);

        for index in 0..self.blip_state.len() {
            // The new blip state is written completely from scratch using the blips
            self.blip_state[index].blip_index = None;
        }

        for (block_index, (block_pos, _placed_block)) in self.machine.blocks.data.iter() {
            Self::update_block_wind_state(
                block_index,
                block_pos,
                &self.machine.blocks.indices,
                &self.machine.blocks.data,
                &self.old_wind_state,
                &mut self.wind_state,
            );
        }

        for (_block_index, (_block_pos, placed_block)) in self.machine.blocks.data.iter_mut() {
            Self::update_block(placed_block);
        }

        Self::update_blips(
            &self.machine.blocks.indices,
            &self.wind_state,
            &self.old_blip_state,
            &mut self.blip_state,
            &mut self.machine.blocks.data,
            &mut self.blips,
        );

        self.check_consistency();

        for (_block_index, (block_pos, placed_block)) in self.machine.blocks.data.iter_mut() {
            Self::update_block_blip_state(
                self.cur_tick,
                block_pos,
                placed_block,
                &self.machine.blocks.indices,
                &mut self.blip_state,
                &mut self.blips,
            );
        }

        self.check_consistency();

        self.cur_tick += 1;
    }

    fn update_block_wind_state(
        block_index: usize,
        block_pos: &Point3,
        block_ids: &Grid3<Option<BlockIndex>>,
        block_data: &VecOption<(Point3, PlacedBlock)>,
        old_wind_state: &[WindState],
        wind_state: &mut Vec<WindState>,
    ) {
        let placed_block = &block_data[block_index].1;

        debug!(
            "wind: {:?} with {:?}",
            placed_block.block, old_wind_state[block_index]
        );

        let dir_y_neg = placed_block.rotated_dir_xy(Dir3::Y_NEG);

        match placed_block.block {
            Block::WindSource => {
                for dir in &Dir3::ALL {
                    let neighbor_pos = *block_pos + dir.to_vector();
                    let neighbor_index = block_ids.get(&neighbor_pos);
                    if let Some(Some(neighbor_index)) = neighbor_index {
                        if block_data[*neighbor_index].1.has_wind_hole_in(dir.invert()) {
                            wind_state[*neighbor_index].wind_in[dir.invert().to_index()] = true;
                        }
                    }
                }
            }
            Block::BlipWindSource { activated } => {
                for dir in &Dir3::ALL {
                    if *dir == dir_y_neg {
                        // Don't put wind in the direction of our blip button
                        continue;
                    }

                    let neighbor_pos = *block_pos + dir.to_vector();
                    let neighbor_index = block_ids.get(&neighbor_pos);
                    if let Some(Some(neighbor_index)) = neighbor_index {
                        if block_data[*neighbor_index].1.has_wind_hole_in(dir.invert()) {
                            wind_state[*neighbor_index].wind_in[dir.invert().to_index()] =
                                activated;
                        }
                    }
                }

                // Note: activated will be set to false in the same tick in
                // `update_block`.
            }
            _ => {
                let any_in = placed_block
                    .wind_holes_in()
                    .iter()
                    .any(|dir| old_wind_state[block_index].wind_in(*dir));

                debug!(
                    "wind holes {:?}, rot {}",
                    placed_block.wind_holes(),
                    placed_block.rotation_xy
                );
                for dir in &placed_block.wind_holes_out() {
                    let neighbor_pos = *block_pos + dir.to_vector();

                    debug!("check wind guy {:?} at {:?}", block_pos, neighbor_pos);
                    if let Some(Some(neighbor_index)) = block_ids.get(&neighbor_pos) {
                        let neighbor_in_flow = if any_in {
                            !old_wind_state[block_index].wind_in[dir.to_index()]
                                && block_data[*neighbor_index].1.has_wind_hole_in(dir.invert())
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

    fn update_block(block: &mut PlacedBlock) {
        match block.block {
            Block::BlipWindSource { ref mut activated } => {
                *activated = false;
            }
            Block::BlipSpawn {
                ref mut activated, ..
            } => {
                *activated = None;
            }
            Block::BlipDuplicator {
                ref mut activated, ..
            } => {
                *activated = None;
            }
            _ => (),
        }
    }

    pub(in crate::exec) fn try_spawn_blip(
        invert: bool,
        kind: BlipKind,
        pos: &Point3,
        block_ids: &Grid3<Option<BlockIndex>>,
        blip_state: &mut Vec<BlipState>,
        blips: &mut VecOption<Blip>,
    ) -> bool {
        if let Some(Some(output_index)) = block_ids.get(&pos) {
            if let Some(blip_index) = blip_state[*output_index].blip_index {
                if invert {
                    debug!("removing blip {} at {:?}", blip_index, pos);
                    //blips.remove(blip_index);
                    blips[blip_index].status = BlipStatus::Dying;
                    blip_state[*output_index].blip_index = None;
                }

                false
            } else {
                debug!("spawning blip at {:?}", pos);

                let blip = Blip {
                    kind,
                    pos: *pos,
                    old_move_dir: None,
                    moved: true, // apply effects for entering block in next frame
                    status: BlipStatus::Spawning,
                };
                blip_state[*output_index].blip_index = Some(blips.add(blip));

                true
            }
        } else {
            false
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

        for (blip_index, blip) in blips.iter() {
            if blip.status == BlipStatus::Dying {
                remove_indices.push(blip_index)
            }
        }

        for &remove_index in &remove_indices {
            blips.remove(remove_index);
        }

        remove_indices.clear();

        for (blip_index, blip) in blips.iter_mut() {
            if blip.status == BlipStatus::Spawning {
                blip.status = BlipStatus::Existing;
            }

            let block_index = block_ids.get(&blip.pos);

            // Don't consider blips that are to be removed in the current tick
            if remove_indices.contains(&blip_index) {
                // TODO: The above check could be inefficient; consider using a
                //       boolean vector.
                continue;
            }

            if let Some(Some(block_index)) = block_index {
                debug!(
                    "blip at {:?}: {:?} vs {:?}",
                    blip.pos, old_blip_state[*block_index].blip_index, blip_index,
                );

                // For each block, we store the BlipIndex of the blip currently in it.
                // Check that this mapping is consistent.
                assert_eq!(old_blip_state[*block_index].blip_index, Some(blip_index));

                Self::update_blip(
                    *block_index,
                    blip_index,
                    blip,
                    block_ids,
                    wind_state,
                    blip_state,
                    block_data,
                    &mut remove_indices,
                );
            } else {
                // Out of bounds.
                // TODO: Can this happen?
                debug!("will mark blip {} as dead due to out-of-bounds", blip_index);
                remove_indices.push(blip_index);
            };
        }

        for remove_index in remove_indices {
            if blips.contains(remove_index) {
                let pos = blips[remove_index].pos;

                debug!("marking blip {} as dead at pos {:?}", remove_index, pos);

                if let Some(Some(block_index)) = block_ids.get(&pos) {
                    if blip_state[*block_index].blip_index == Some(remove_index) {
                        blip_state[*block_index].blip_index = None;
                    }
                }

                blips[remove_index].status = BlipStatus::Dying;
                //blips.remove(remove_index);
            }
        }
    }

    fn check_consistency(&self) {
        let block_indices = &self.machine.blocks.indices;
        let block_data = &self.machine.blocks.data;

        for (block_index, (block_pos, _placed_block)) in block_data.iter() {
            debug_assert_eq!(
                block_indices[*block_pos],
                Some(block_index),
                "block with index {} in data has position {:?}, but index grid stores {:?} at that position",
                block_index,
                block_pos,
                block_indices[*block_pos],
            );
        }

        for (blip_index, blip) in self.blips.iter() {
            let block_index = block_indices[blip.pos].unwrap();
            let blip_index_in_block = self.blip_state[block_index].blip_index;

            if blip.status != BlipStatus::Dying {
                debug_assert_eq!(
                    blip_index_in_block,
                    Some(blip_index),
                    "blip with index {} has position {:?}, which has block index {}, but blip state stores blip index {:?} at that position",
                    blip_index,
                    blip.pos,
                    block_index,
                    blip_index_in_block,
                );
            }
        }
    }

    fn get_blip_move_dir(
        blip: &Blip,
        placed_block: &PlacedBlock,
        block_ids: &Grid3<Option<BlockIndex>>,
        block_data: &VecOption<(Point3, PlacedBlock)>,
        wind_state: &[WindState],
    ) -> Option<Dir3> {
        // To determine if it is possible for the blip to move in a certain
        // direction, we check the in flow of the neighboring block in that
        // direction.
        let can_move_to_dir = |dir: &Dir3| {
            // TODO: At some point, we'll need to precompute neighbor
            //       indices.

            let neighbor_index = block_ids.get(&(blip.pos + dir.to_vector()));
            let neighbor_in = if let Some(Some(neighbor_index)) = neighbor_index {
                wind_state[*neighbor_index].wind_in(dir.invert())
                    && block_data[*neighbor_index].1.has_move_hole(dir.invert())
            } else {
                false
            };

            neighbor_in && placed_block.has_move_hole(*dir)
        };

        // Note that there might be multiple directions the blip can move in.
        // If the blip already is moving, it will always prefer to keep moving
        // in that direction. If that is not possible, it will try directions
        // clockwise to its current direction. (TODO)
        Dir3::ALL.iter().cloned().find(can_move_to_dir)
    }

    fn update_blip(
        block_index: usize,
        blip_index: usize,
        blip: &mut Blip,
        block_ids: &Grid3<Option<BlockIndex>>,
        wind_state: &[WindState],
        blip_state: &mut Vec<BlipState>,
        block_data: &mut VecOption<(Point3, PlacedBlock)>,
        remove_indices: &mut Vec<BlipIndex>,
    ) {
        assert_eq!(block_data[block_index].0, blip.pos);

        if blip.moved {
            // Blip moved in last tick. Apply effects of entering the new
            // block.
            blip.moved = false;

            let placed_block = &mut block_data[block_index].1;
            let remove = Self::on_blip_enter_block(blip, placed_block);
            if remove {
                // Effect of new block causes blip to be removed
                debug!(
                    "will mark blip {} as dead due to block {:?} effect",
                    blip_index, placed_block,
                );

                // Disable interpolation for this blip
                blip.old_move_dir = None;

                remove_indices.push(blip_index);
                return;
            }
        }

        let placed_block = block_data[block_index].1.clone();
        let out_dir =
            Self::get_blip_move_dir(blip, &placed_block, block_ids, block_data, wind_state);
        let new_pos = if let Some(out_dir) = out_dir {
            Self::on_blip_leave_block(blip, out_dir, &mut block_data[block_index].1);
            blip.moved = true;

            blip.pos + out_dir.to_vector()
        } else {
            blip.pos
        };

        debug!(
            "moving blip {} from {:?} to {:?}",
            blip_index, blip.pos, new_pos
        );

        let new_block_index = block_ids.get(&new_pos);

        // Remember the movement direction for the next tick and for visual
        // purposes.
        blip.old_move_dir = out_dir;

        if let Some(Some(new_block_index)) = new_block_index {
            blip.pos = new_pos;

            if let Some(new_block_blip_index) = blip_state[*new_block_index].blip_index {
                // We cannot have two blips in the same block. Note
                // that if more than two blips move into the same
                // block, the same blip will be added multiple times
                // into `remove_indices`. This is fine, since we don't
                // spawn any blips in this function, so the indices
                // stay valid.
                debug!(
                    "{} bumped into {}, will mark as dead",
                    blip_index, new_block_blip_index
                );

                remove_indices.push(new_block_blip_index);
                remove_indices.push(blip_index);
            } else {
                blip_state[*new_block_index].blip_index = Some(blip_index);
            }
        } else {
            // We are on the grid, but there is no block at our position
            // -> remove blip
            debug!("will mark blip {} as dead due to no block", blip_index);
            remove_indices.push(blip_index);
        }
    }

    fn on_blip_leave_block(_blip: &Blip, _dir: Dir3, placed_block: &mut PlacedBlock) {
        match placed_block.block {
            Block::PipeSplitXY { open_move_hole_y } => {
                placed_block.block = Block::PipeSplitXY {
                    open_move_hole_y: open_move_hole_y.invert(),
                };
            }
            _ => (),
        }
    }

    fn on_blip_enter_block(blip: &Blip, new_placed_block: &mut PlacedBlock) -> bool {
        match new_placed_block.block {
            Block::BlipDuplicator {
                kind,
                ref mut activated,
                ..
            } => {
                // TODO: Resolve possible race condition in blip
                //       duplicator. If two blips of different
                //       kind race into the duplicator, the output
                //       kind depends on the order of blip
                //       evaluation.
                if kind == None || kind == Some(blip.kind) {
                    *activated = Some(blip.kind);
                }

                // Remove blip
                true
            }
            Block::BlipWindSource { ref mut activated } => {
                *activated = true;

                // Remove blip
                true
            }
            _ => false,
        }
    }

    fn update_block_blip_state(
        cur_tick: TickNum,
        block_pos: &Point3,
        placed_block: &mut PlacedBlock,
        block_ids: &Grid3<Option<BlockIndex>>,
        blip_state: &mut Vec<BlipState>,
        blips: &mut VecOption<Blip>,
    ) {
        let dir_x_pos = placed_block.rotated_dir_xy(Dir3::X_POS);
        let dir_x_neg = placed_block.rotated_dir_xy(Dir3::X_NEG);

        match placed_block.block {
            Block::BlipSpawn {
                kind,
                ref mut num_spawns,
                ref mut activated,
            } => {
                *activated = None;

                if num_spawns.map_or(true, |n| n > 0) {
                    let output_pos = *block_pos + dir_x_pos.to_vector();
                    let did_spawn = Self::try_spawn_blip(
                        false,
                        kind,
                        &output_pos,
                        block_ids,
                        blip_state,
                        blips,
                    );

                    *num_spawns = num_spawns.map_or(None, |n| Some(n - 1));
                    if did_spawn {
                        *activated = Some(cur_tick);
                    }
                }
            }
            Block::BlipDuplicator { activated, .. } => {
                // TODO: Only allow activating with specific kind?
                if let Some(kind) = activated {
                    Self::try_spawn_blip(
                        true,
                        kind,
                        &(*block_pos + dir_x_pos.to_vector()),
                        block_ids,
                        blip_state,
                        blips,
                    );
                    Self::try_spawn_blip(
                        true,
                        kind,
                        &(*block_pos + dir_x_neg.to_vector()),
                        block_ids,
                        blip_state,
                        blips,
                    );
                }
            }
            _ => {}
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
