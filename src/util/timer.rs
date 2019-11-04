use std::ops::AddAssign;
use std::time::Duration;

use floating_duration::TimeAsFloat;

pub fn secs_to_duration(t: f32) -> Duration {
    debug_assert!(t >= 0.0, "secs_to_duration passed a negative number");

    let seconds = t.trunc();
    let nanos = t.fract() * 1e9;
    Duration::new(seconds as u64, nanos as u32)
}

pub fn hz_to_period(hz: f32) -> Duration {
    secs_to_duration(1.0 / hz)
}

/// A timer that can be used to trigger events that happen periodically.
#[derive(Debug, Clone, Copy)]
pub struct Timer {
    pub period: Duration,
    pub accum: Duration,
}

impl Timer {
    pub fn new(period: Duration) -> Timer {
        Timer {
            period,
            accum: Default::default(),
        }
    }

    pub fn period(&self) -> Duration {
        self.period
    }

    /// Has the timer accumulated enough time for one period?
    /// If yes, subtract the period from the timer.
    pub fn trigger(&mut self) -> bool {
        if self.accum >= self.period {
            self.accum = self.accum.checked_sub(self.period).unwrap();
            true
        } else {
            false
        }
    }

    /// Returns the number of periods the timer has accumulated.
    /// Subtracts the periods from the timer.
    pub fn trigger_n(&mut self) -> usize {
        let accum = self.accum.as_fractional_secs() as f32;
        let period = self.period.as_fractional_secs() as f32;
        let n = (accum / period).floor();

        self.accum = secs_to_duration(accum - period * n);

        n as usize
    }

    /// Has the timer accumulated enough time for one period?
    /// If yes, reset the timer to zero.
    pub fn trigger_reset(&mut self) -> bool {
        if self.accum >= self.period {
            self.accum = Duration::default();
            true
        } else {
            false
        }
    }

    /// Reset the accumulated time. Returns true if enough time has passed for one period.
    pub fn reset(&mut self) -> bool {
        let trigger = self.accum >= self.period;
        self.accum = Duration::default();
        trigger
    }

    /// Percentual progress until the next period.
    pub fn progress(&self) -> f32 {
        (self.accum.as_fractional_secs() / self.period.as_fractional_secs()) as f32
    }

    pub fn accum(&self) -> Duration {
        self.accum
    }
}

impl AddAssign<Duration> for Timer {
    fn add_assign(&mut self, other: Duration) {
        self.accum = self.accum.checked_add(other).unwrap();
    }
}
