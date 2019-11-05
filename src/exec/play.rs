use std::time::Duration;

use glium::glutin::{ElementState, VirtualKeyCode, WindowEvent};
use imgui::{im_str, ImString};
use log::info;
use nalgebra as na;

use crate::util::timer::{self, Timer};

pub const TICKS_PER_SEC_SLOW: f32 = 0.5;
pub const TICKS_PER_SEC_NORMAL: f32 = 1.0;
pub const TICKS_PER_SEC_FAST: f32 = 2.0;
pub const TICKS_PER_SEC_FASTER: f32 = 4.0;
pub const TICKS_PER_SEC_FASTEST: f32 = 8.0;

pub const MAX_TICKS_PER_UPDATE: usize = 100;

#[derive(Debug, Clone)]
pub struct Config {
    pub play_pause_key: VirtualKeyCode,
    pub stop_key: VirtualKeyCode,
    pub single_tick_key: VirtualKeyCode,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            play_pause_key: VirtualKeyCode::Space,
            stop_key: VirtualKeyCode::Escape,
            single_tick_key: VirtualKeyCode::F,
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
    pub fn zero() -> Self {
        Self {
            num_ticks_passed: 0,
            next_tick_timer: Timer::new(timer::hz_to_period(TICKS_PER_SEC_NORMAL)),
        }
    }

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

    pub fn is_playing(&self) -> bool {
        match self {
            Status::Playing { .. } => true,
            _ => false,
        }
    }

    pub fn is_paused(&self) -> bool {
        match self {
            Status::Paused { .. } => true,
            _ => false,
        }
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
            ticks_per_sec: TICKS_PER_SEC_NORMAL,
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
                new_time.next_tick_timer.set_period(tick_period);
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

    pub fn ui(&mut self, window_size: na::Vector2<f32>, status: Option<&Status>, ui: &imgui::Ui) {
        let bg_alpha = 0.8;
        let button_w = 60.0;
        let button_h = 25.0;

        let is_stopped = status.is_none();
        let is_playing = status.map_or(false, |status| status.is_playing());
        let is_paused = status.map_or(false, |status| status.is_paused());
        let playing_period = if let Some(Status::Playing { time, .. }) = status {
            Some(time.next_tick_timer.period())
        } else {
            None
        };

        imgui::Window::new(im_str!("Play"))
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
                    .selected(is_stopped)
                    .size([27.5, 0.0]);
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
                let selectable = imgui::Selectable::new(im_str!("⏸"))
                    .selected(is_paused)
                    .disabled(is_stopped)
                    .size([27.5, 0.0]);
                if selectable.build(ui) && is_playing {
                    self.play_pause_pressed = true;
                }
                if ui.is_item_hovered() {
                    let text = format!(
                        "Pause machine execution.\n\nShortcut: {:?}",
                        self.config.play_pause_key
                    );
                    ui.tooltip(|| ui.text(&ImString::new(text)));
                }

                let speeds = [
                    (im_str!("▶"), TICKS_PER_SEC_NORMAL, 27.5),
                    (im_str!("▶▶"), TICKS_PER_SEC_FAST, 35.0),
                    (im_str!("▶▶▶"), TICKS_PER_SEC_FASTEST, 42.5),
                ];

                for (symbol, ticks_per_sec, size) in speeds.into_iter() {
                    ui.same_line(0.0);
                    let selectable = imgui::Selectable::new(symbol)
                        .selected(playing_period == Some(timer::hz_to_period(*ticks_per_sec)))
                        .size([*size, 0.0]);
                    if selectable.build(ui) {
                        self.ticks_per_sec = *ticks_per_sec;
                        if !is_playing {
                            self.play_pause_pressed = true;
                        }
                    }
                }

                ui.set_window_font_scale(1.0);
            });
    }
}
