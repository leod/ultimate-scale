use crate::exec::{Activation, Exec};
use crate::machine::grid::DirMap3;
use crate::machine::{Block, BlockIndex};

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
        let wind_out = DirMap3::from_fn(|dir| {
            WindLife::from_states(
                exec.blocks().wind_out[block_index][dir],
                exec.next_blocks().wind_out[block_index][dir],
            )
        });

        let activation = exec.blocks().activation[block_index];
        let next_activation = exec.next_blocks().activation[block_index];

        let out_deadend = exec.neighbor_map()[block_index].map(|dir, &neighbor_index| {
            if let Some(neighbor_index) = neighbor_index {
                let neighbor_block = exec.machine().block_at_index(neighbor_index);

                if !neighbor_block.has_wind_hole_in(dir.invert(), activation.is_some()) {
                    // If neighboring block has no wind connection in this
                    // direction, we won't show wind.
                    if neighbor_block.is_pipe() {
                        Some(WindDeadend::Space)
                    } else {
                        Some(WindDeadend::Block)
                    }
                } else if let Block::Air = neighbor_block {
                    // Show wind only partially into air
                    Some(WindDeadend::Space)
                } else {
                    None
                }
            } else {
                // No block, will show wind partially.
                Some(WindDeadend::Space)
            }
        });

        Self {
            wind_out,
            out_deadend,
            activation,
            next_activation,
        }
    }

    pub fn num_alive_out(&self) -> usize {
        self.wind_out
            .values()
            .filter(|anim| anim.is_alive())
            .count()
    }
}
