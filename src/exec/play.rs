use std::fmt;
use std::time::Duration;

use glium::glutin::{ElementState, VirtualKeyCode, WindowEvent};
use imgui::{im_str, ImString};
use log::info;
use nalgebra as na;

use crate::util::timer::{self, Timer};

/// Possible choices in the UI for number of ticks per second to play.
/// (Specifying these as strings instead of floats here is easier than figuring
///  out how to format floats nicely.)
pub const TICKS_PER_SEC_CHOICES: &[&str] = &[
    "0.25", "0.5", "1", "2", "4", "8", "16", "32", "64", "128", "256", "512",
];

pub const MAX_TICKS_PER_UPDATE: usize = 1024;

#[derive(Debug, Clone)]
pub struct Config {
    pub play_pause_key: VirtualKeyCode,
    pub stop_key: VirtualKeyCode,
    pub faster_key: VirtualKeyCode,
    pub slower_key: VirtualKeyCode,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            play_pause_key: VirtualKeyCode::Space,
            stop_key: VirtualKeyCode::Escape,
            faster_key: VirtualKeyCode::Add,
            slower_key: VirtualKeyCode::Subtract,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub struct TickTime {
    /// Number of ticks that have already passed since starting the simulation.
    pub num_ticks_passed: usize,

    /// Progress in starting the next tick.
    pub next_tick_timer: Timer,
}

impl TickTime {
    pub fn zero() -> Self {
        Self {
            num_ticks_passed: 0,
            next_tick_timer: Timer::new(timer::hz_to_period(1.0)),
        }
    }

    pub fn to_f32(&self) -> f32 {
        self.num_ticks_passed as f32 + self.next_tick_timer.progress()
    }

    pub fn tick_progress(&self) -> f32 {
        self.next_tick_timer.progress()
    }
}

impl fmt::Display for TickTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2}", self.to_f32())
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

        /// Time of the last update.
        prev_time: Option<TickTime>,

        /// TickTime since starting the simulation.
        time: TickTime,
    },

    /// Playback is paused.
    Paused {
        /// TickTime since starting the simulation.
        time: TickTime,
    },

    /// Playback is finished.
    Finished {
        /// TickTime since starting the simulation.
        time: TickTime,
    },
}

impl Status {
    pub fn time(&self) -> &TickTime {
        match self {
            Status::Playing { time, .. } => time,
            Status::Paused { time, .. } => time,
            Status::Finished { time, .. } => time,
        }
    }

    pub fn tick_progress(&self) -> f32 {
        self.time().tick_progress()
    }

    pub fn is_paused(&self) -> bool {
        match self {
            Status::Paused { .. } => true,
            _ => false,
        }
    }

    pub fn is_finished(&self) -> bool {
        match self {
            Status::Finished { .. } => true,
            _ => false,
        }
    }
}

pub struct Play {
    config: Config,
    ticks_per_sec_index: usize,

    play_pause_pressed: bool,
    stop_pressed: bool,
}

impl Play {
    pub fn new(config: &Config) -> Self {
        Play {
            config: config.clone(),
            ticks_per_sec_index: 2,
            play_pause_pressed: false,
            stop_pressed: false,
        }
    }

    pub fn update_status(&mut self, dt: Duration, status: Option<&Status>) -> Option<Status> {
        let play_pause_pressed = self.play_pause_pressed;
        let stop_pressed = self.stop_pressed;

        self.play_pause_pressed = false;
        self.stop_pressed = false;

        // Can unwrap here since TICKS_PER_SEC_CHOICES contains
        // only valid floats.
        let tick_period = timer::hz_to_period(
            TICKS_PER_SEC_CHOICES[self.ticks_per_sec_index]
                .parse()
                .unwrap(),
        );

        match &status {
            Some(Status::Playing { time, .. }) if play_pause_pressed => {
                info!("Pausing exec at time {}", time);
                Some(Status::Paused { time: time.clone() })
            }
            Some(Status::Playing { .. }) if stop_pressed => None,
            Some(Status::Playing { time, .. }) => {
                // Set the Timer's period first, since this may change
                // how many ticks are run in the current update.
                // This also ensures that `Timer::progress` will be between
                // 0 and 1.
                let mut new_time = time.clone();
                new_time.next_tick_timer.set_period(tick_period);
                new_time.next_tick_timer += dt;

                let num_ticks_since_last_update = new_time.next_tick_timer.trigger_n();
                new_time.num_ticks_passed += num_ticks_since_last_update.min(MAX_TICKS_PER_UPDATE);

                Some(Status::Playing {
                    num_ticks_since_last_update,
                    prev_time: Some(time.clone()),
                    time: new_time,
                })
            }
            Some(Status::Paused { time }) if play_pause_pressed => {
                info!("Resuming exec at time {}", time);
                Some(Status::Playing {
                    num_ticks_since_last_update: 0,
                    prev_time: None,
                    time: time.clone(),
                })
            }
            Some(Status::Paused { time }) if stop_pressed => {
                info!("Stopping exec at time {}", time);
                None
            }
            Some(Status::Finished { time }) if stop_pressed => {
                info!("Stopping exec at time {}", time);
                None
            }
            Some(Status::Finished { time }) => {
                // Advance tick timer even when finished, so that we see the
                // interpolation into the last state. Tick timer is only
                // advanced within the current tick though.
                // We only advance through the tick partially, so that the
                // last blips are still seen at the stop. This is especially
                // useful to see why a level was failed.
                let progress_limit = 0.7;

                let mut new_time = time.clone();

                if time.tick_progress() < progress_limit {
                    new_time.next_tick_timer.set_period(tick_period);
                    new_time.next_tick_timer += dt;
                }

                if new_time.tick_progress() > progress_limit {
                    new_time.next_tick_timer.set_progress(progress_limit);
                }

                Some(Status::Finished { time: new_time })
            }
            None if play_pause_pressed => {
                info!("Starting exec");
                Some(Status::Playing {
                    num_ticks_since_last_update: 0,
                    prev_time: None,
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
        } else if keycode == self.config.faster_key {
            if self.ticks_per_sec_index + 1 < TICKS_PER_SEC_CHOICES.len() {
                self.ticks_per_sec_index += 1;
            }
        } else if keycode == self.config.slower_key {
            if self.ticks_per_sec_index > 0 {
                self.ticks_per_sec_index -= 1;
            }
        }
    }

    pub fn ui(&mut self, window_size: na::Vector2<f32>, status: Option<&Status>, ui: &imgui::Ui) {
        let bg_alpha = 0.8;

        let is_stopped = status.is_none();
        let is_paused = status.map_or(false, |status| status.is_paused());
        let is_finished = status.map_or(false, |status| status.is_finished());

        let title = format!(
            "Play @ {}Hz###Play",
            TICKS_PER_SEC_CHOICES[self.ticks_per_sec_index]
        );
        imgui::Window::new(&ImString::new(title))
            .horizontal_scrollbar(true)
            .movable(false)
            .always_auto_resize(true)
            .position(
                [window_size.x / 2.0, window_size.y - 10.0],
                imgui::Condition::Always,
            )
            .position_pivot([0.5, 1.0])
            .bg_alpha(bg_alpha)
            .build(&ui, || {
                ui.set_window_font_scale(1.5);

                let selectable = imgui::Selectable::new(im_str!("⏹"))
                    .disabled(is_stopped)
                    .size([21.0, 0.0]);
                if selectable.build(ui) {
                    self.stop_pressed = true;
                }
                if ui.is_item_hovered() {
                    let text = format!(
                        "Stop machine execution.\n\nShortcut: {:?}",
                        self.config.stop_key
                    );
                    ui.tooltip(|| ui.text(&ImString::new(text)));
                }

                ui.same_line(0.0);

                let symbol = if is_paused || is_stopped {
                    im_str!("▶")
                } else {
                    im_str!("⏸")
                };

                let selectable = imgui::Selectable::new(symbol)
                    .disabled(is_finished)
                    .size([21.0, 0.0]);
                if selectable.build(ui) {
                    self.play_pause_pressed = true;
                }
                if ui.is_item_hovered() {
                    let text = format!(
                        "Run/pause machine execution.\n\nShortcut: {:?}",
                        self.config.play_pause_key
                    );
                    ui.tooltip(|| ui.text(&ImString::new(text)));
                }

                ui.same_line_with_spacing(0.0, 30.0);

                let selectable = imgui::Selectable::new(im_str!("-"))
                    .disabled(self.ticks_per_sec_index == 0)
                    .size([15.0, 0.0]);
                if selectable.build(ui) {
                    if self.ticks_per_sec_index > 0 {
                        self.ticks_per_sec_index -= 1;
                    }
                }
                if ui.is_item_hovered() {
                    let text = format!(
                        "Slow down execution.\n\nShortcut: {:?}",
                        self.config.slower_key
                    );
                    ui.tooltip(|| ui.text(&ImString::new(text)));
                }

                ui.same_line(0.0);
                let selectable = imgui::Selectable::new(im_str!("+"))
                    .disabled(self.ticks_per_sec_index + 1 == TICKS_PER_SEC_CHOICES.len())
                    .size([15.0, 0.0]);
                if selectable.build(ui) {
                    if self.ticks_per_sec_index + 1 < TICKS_PER_SEC_CHOICES.len() {
                        self.ticks_per_sec_index += 1;
                    }
                }
                if ui.is_item_hovered() {
                    let text = format!(
                        "Speed up execution.\n\nShortcut: {:?}",
                        self.config.faster_key
                    );
                    ui.tooltip(|| ui.text(&ImString::new(text)));
                }

                ui.set_window_font_scale(1.0);
            });
    }
}
