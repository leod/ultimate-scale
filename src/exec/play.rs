use std::time::Duration;

use log::info;

use glium::glutin::{ElementState, VirtualKeyCode, WindowEvent};

use crate::util::timer::{self, Timer};

#[derive(Debug, Clone)]
pub struct Config {
    pub play_pause_key: VirtualKeyCode,
    pub stop_key: VirtualKeyCode,
    pub single_tick_key: VirtualKeyCode,

    pub default_ticks_per_sec: f32,
    pub max_ticks_per_update: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            play_pause_key: VirtualKeyCode::Space,
            stop_key: VirtualKeyCode::Escape,
            single_tick_key: VirtualKeyCode::F,

            default_ticks_per_sec: 0.5,
            max_ticks_per_update: 100,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TickTime {
    /// Number of ticks that have already passed since starting the simulation.
    pub num_ticks_passed: usize,

    /// Progress in starting the next tick.
    pub next_tick_timer: Timer,
}

impl TickTime {
    pub fn as_f32(&self) -> f32 {
        self.num_ticks_passed as f32 + self.next_tick_timer.progress()
    }

    pub fn tick_progress(&self) -> f32 {
        self.next_tick_timer.progress()
    }
}

/// A status for execution playback.
#[derive(Debug, Clone)]
pub enum Status {
    /// Playback is advancing.
    Playing {
        /// Number of execution ticks that have passed run since the last
        /// update.
        num_ticks_since_last_update: usize,

        /// TickTime since starting the simulation.
        time: TickTime,
    },

    /// Playback is paused.
    Paused {
        /// TickTime since starting the simulation.
        time: TickTime,
    },
}

impl Status {
    pub fn time(&self) -> &TickTime {
        match self {
            Status::Playing { time, .. } => time,
            Status::Paused { time, .. } => time,
        }
    }

    pub fn tick_progress(&self) -> f32 {
        self.time().tick_progress()
    }
}

pub struct Play {
    config: Config,
    ticks_per_sec: f32,

    play_pause_pressed: bool,
    stop_pressed: bool,
}

impl Play {
    pub fn new(config: &Config) -> Self {
        Play {
            config: config.clone(),
            ticks_per_sec: config.default_ticks_per_sec,
            play_pause_pressed: false,
            stop_pressed: false,
        }
    }

    pub fn update_status(&mut self, dt: Duration, status: Option<&Status>) -> Option<Status> {
        let play_pause_pressed = self.play_pause_pressed;
        let stop_pressed = self.stop_pressed;

        self.play_pause_pressed = false;
        self.stop_pressed = false;

        let tick_period = timer::hz_to_period(self.ticks_per_sec);

        match &status {
            Some(Status::Playing { time, .. }) if play_pause_pressed => {
                info!("Pausing exec");
                Some(Status::Paused { time: time.clone() })
            }
            Some(Status::Playing { .. }) if stop_pressed => None,
            Some(Status::Playing { time, .. }) => {
                // Set the Timer's period first, since this may change
                // how many ticks are run in the current update.
                // This also ensures that `Timer::progress` will be between
                // 0 and 1.
                let mut new_time = time.clone();
                new_time.next_tick_timer.period = tick_period;
                new_time.next_tick_timer += dt;

                let num_ticks_since_last_update = new_time.next_tick_timer.trigger_n();
                new_time.num_ticks_passed += num_ticks_since_last_update;

                Some(Status::Playing {
                    num_ticks_since_last_update,
                    time: new_time,
                })
            }
            Some(Status::Paused { time }) if play_pause_pressed => {
                info!("Resuming exec");
                Some(Status::Playing {
                    num_ticks_since_last_update: 0,
                    time: time.clone(),
                })
            }
            Some(Status::Paused { .. }) if stop_pressed => {
                info!("Stopping exec");
                None
            }
            None if play_pause_pressed => {
                info!("Starting exec");
                Some(Status::Playing {
                    num_ticks_since_last_update: 0,
                    time: TickTime {
                        num_ticks_passed: 0,
                        next_tick_timer: Timer::new(tick_period),
                    },
                })
            }
            other => other.cloned(),
        }
    }

    pub fn on_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { input, .. } => {
                if input.state == ElementState::Pressed {
                    if let Some(keycode) = input.virtual_keycode {
                        self.on_key_press(keycode);
                    }
                }
            }
            _ => (),
        }
    }

    fn on_key_press(&mut self, keycode: VirtualKeyCode) {
        if keycode == self.config.play_pause_key {
            self.play_pause_pressed = true;
        } else if keycode == self.config.stop_key {
            self.stop_pressed = true;
        } else if keycode == self.config.single_tick_key {
            // TODO
        }
    }
}
