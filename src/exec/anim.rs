use crate::exec::{Exec, Activation};
use crate::machine::grid::{Dir3, DirMap3};
use crate::machine::BlockIndex;

/// Stages in the lifecycle of wind in some direction in a block. Used for
/// animation purposes.
#[derive(Debug, Copy, Clone)]
pub enum WindLife {
    /// No wind in last or current tick.
    None,

    /// This wind did not exist last tick and is now appearing.
    Appearing,

    /// This wind existed in the last tick and still exists.
    Existing,

    /// This wind existed in the last tick and now doesn't anymore.
    Disappearing,
}

/// Wind going into a deadend is shown differently.
#[derive(Debug, Copy, Clone)]
pub enum WindDeadend {
    Block,
    Space,
}

impl WindLife {
    /// Returns the WindLife given the flow state in previous and current tick.
    pub fn from_states(old: bool, new: bool) -> Self {
        match (old, new) {
            (false, false) => WindLife::None,
            (false, true) => WindLife::Appearing,
            (true, true) => WindLife::Existing,
            (true, false) => WindLife::Disappearing,
        }
    }

    /// Is wind flowing?
    pub fn is_alive(self) -> bool {
        match self {
            WindLife::None => false,
            _ => true,
        }
    }
}

/// Animation state for wind in all directions in a block. Used for animation
/// purposes. Also stores activation state.
pub struct AnimState {
    pub wind_out: DirMap3<WindLife>,
    pub out_deadend: DirMap3<Option<WindDeadend>>,
    pub activation: Activation,
    pub next_activation: Activation,
}

impl AnimState {
    /// Returns the AnimState of one block based on the previous and the
    /// current simulation WindState.
    pub fn from_exec_block(exec: &Exec, block_index: BlockIndex) -> Self {
        let machine = exec.machine();

        let wind_out = DirMap3::from_fn(|dir| {
            WindLife::from_states(
                exec.blocks().wind_out[block_index][dir],
                exec.next_blocks().wind_out[block_index][dir],
            )
        });

        let out_deadend = exec.neighbor_map()[block_index].map(|(dir, neighbor_index)| {
            if let Some(neighbor_index) = neighbor_index {
                let neighbor_block = exec.machine().block_at_index(neighbor_index);

                if !neighbor_block.has_wind_hole_in(dir.invert()) {
                    // If neighboring block has no wind connection in this
                    // direction, we won't show wind.
                    Some(WindDeadend::Block)
                } else {
                    None
                }
            } else {
                // No block, will show wind partially.
                Some(WindDeadend::Space)
            }
        });

        WindAnimState {
            wind_out,
            out_deadend,
            activation: exec.blocks().activation[block_index],
            next_activation: exec.blocks().next_activation[block_index],
        }
    }

    pub fn wind_out(&self, dir: Dir3) -> WindLife {
        self.wind_out[dir.to_index()]
    }

    pub fn out_deadend(&self, dir: Dir3) -> Option<WindDeadend> {
        self.out_deadend[dir.to_index()]
    }

    pub fn num_alive_in(&self) -> usize {
        self.wind_in.iter().filter(|anim| anim.is_alive()).count()
    }

    pub fn num_alive_out(&self) -> usize {
        self.wind_out.iter().filter(|anim| anim.is_alive()).count()
    }
}
