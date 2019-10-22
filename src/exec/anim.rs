use crate::exec::WindState;
use crate::machine::grid::Dir3;

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
}

/// Animation state for incoming wind in all directions in a block. Used for
/// animation purposes.
pub struct WindAnimState {
    pub wind_in: [WindLife; Dir3::NUM_INDICES],
}

impl WindAnimState {
    /// Returns the WindAnimState of one block based on the previous and the
    /// current simulation WindState.
    pub fn from_states(old: &WindState, cur: &WindState) -> Self {
        let mut wind_in = [WindLife::None; Dir3::NUM_INDICES];

        for &dir in &Dir3::ALL {
            wind_in[dir.to_index()] = WindLife::from_states(old.wind_in(dir), cur.wind_in(dir));
        }

        WindAnimState { wind_in }
    }

    pub fn wind_in(&self, dir: Dir3) -> WindLife {
        self.wind_in[dir.to_index()]
    }
}
