mod draw;
mod update;

use std::time::Duration;

use coarse_prof::profile;
use floating_duration::TimeAsFloat;
use log::info;
use nalgebra as na;

use glium::glutin;

use rendology::{Camera, Light};

use crate::config::{self, Config};
use crate::edit::Editor;
use crate::edit_camera_view::{EditCameraView, EditCameraViewInput};
use crate::exec::play::{self, Play, TickTime};
use crate::exec::{ExecView, LevelProgress, LevelStatus};
use crate::input_state::InputState;
use crate::machine::Machine;
use crate::render;
use crate::util::stats;

use draw::Draw;
use update::Update;

pub struct Game {
    config: Config,

    update: Update,
    draw: Draw,

    target_size: (u32, u32),
    next_window_events: Vec<(InputState, glutin::WindowEvent)>,

    fps: stats::Variable,
}

impl Game {
    pub fn create<F: glium::backend::Facade>(
        facade: &F,
        config: &Config,
        initial_machine: Machine,
    ) -> Result<Game, draw::CreationError> {
        info!("Creating resources");

        let editor = Editor::new(&config.editor, initial_machine);
        let update = Update::new_editor(config, editor);
        let draw = Draw::create(facade, config)?;

        // TODO: Account for DPI in initialization
        let target_size = config.view.window_size.into();

        Ok(Game {
            config: config.clone(),
            update,
            draw,
            target_size,
            next_window_events: Vec::new(),
            fps: stats::Variable::new(Duration::from_secs(1)),
        })
    }

    pub fn draw<F: glium::backend::Facade, S: glium::Surface>(
        &mut self,
        facade: &F,
        target: &mut S,
    ) -> Result<(), rendology::DrawError> {
        self.target_size = target.get_dimensions();

        Ok(())
    }

    pub fn update(&mut self, dt: Duration, input_state: &InputState) {
        self.fps.record(1.0 / dt.as_secs_f32());

        let window_events = std::mem::replace(&mut self.next_window_events, Vec::new());

        let input = update::Input {
            dt,
            window_events,
            input_state: input_state.clone(),
            target_size: self.target_size,
        };
    }

    pub fn on_event(&mut self, input_state: &InputState, event: &glutin::WindowEvent) {
        self.next_window_events
            .push((input_state.clone(), event.clone()));
    }

    pub fn on_window_resize<F: glium::backend::Facade>(
        &mut self,
        _facade: &F,
        new_window_size: glutin::dpi::LogicalSize,
    ) -> Result<(), draw::CreationError> {
        Ok(())
    }
}
