use crate::exec::{Exec, WindState};
use crate::machine::grid::Dir3;
use crate::machine::{BlockIndex, Machine};

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
/// purposes.
pub struct WindAnimState {
    pub wind_in: [WindLife; Dir3::NUM_INDICES],
    pub wind_out: [WindLife; Dir3::NUM_INDICES],
}

impl WindAnimState {
    /// Returns the WindAnimState of one block based on the previous and the
    /// current simulation WindState.
    pub fn from_exec_block(exec: &Exec, block_index: BlockIndex) -> Self {
        let machine = exec.machine();

        let mut wind_in = [WindLife::None; Dir3::NUM_INDICES];
        let mut wind_out = [WindLife::None; Dir3::NUM_INDICES];

        for &dir in &Dir3::ALL {
            // Incoming wind
            wind_in[dir.to_index()] = WindLife::from_states(
                exec.old_wind_state()[block_index].wind_in(dir),
                exec.wind_state()[block_index].wind_in(dir),
            );

            // Outgoing wind
            let neighbor_pos = machine.block_pos_at_index(block_index) + dir.to_vector();
            let neighbor_block = machine.get_block_at_pos(&neighbor_pos);
            if let Some((neighbor_index, _neighbor_block)) = neighbor_block {
                wind_out[dir.to_index()] = WindLife::from_states(
                    exec.old_wind_state()[neighbor_index].wind_in(dir.invert()),
                    exec.wind_state()[neighbor_index].wind_in(dir.invert()),
                );
            }
        }

        WindAnimState { wind_in, wind_out }
    }

    pub fn wind_in(&self, dir: Dir3) -> WindLife {
        self.wind_in[dir.to_index()]
    }

    pub fn wind_out(&self, dir: Dir3) -> WindLife {
        self.wind_out[dir.to_index()]
    }

    pub fn num_alive_in(&self) -> usize {
        self.wind_in.iter().filter(|anim| anim.is_alive()).count()
    }

    pub fn num_alive_out(&self) -> usize {
        self.wind_out.iter().filter(|anim| anim.is_alive()).count()
    }
}
