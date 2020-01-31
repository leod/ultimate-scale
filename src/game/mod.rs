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
use update::{Update, UpdateRunner};

pub struct Game {
    config: Config,

    update: UpdateRunner,
    draw: Draw,

    target_size: (u32, u32),
    next_window_events: Vec<(InputState, glutin::WindowEvent)>,

    last_output: Option<update::Output>,

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
        let mut update = UpdateRunner::spawn(Update::new_editor(config, editor));
        let draw = Draw::create(facade, config)?;

        // TODO: Account for DPI in initialization
        let target_size = config.view.window_size.into();

        // Kick off the update loop, so that we get our first `update::Output`
        // to draw.
        update.send_input(update::Input {
            dt: Duration::from_secs(0),
            window_events: Vec::new(),
            input_state: InputState::empty(1.0),
            target_size,
        });

        Ok(Game {
            config: config.clone(),
            update,
            draw,
            target_size,
            next_window_events: Vec::new(),
            last_output: None,
            fps: stats::Variable::new(Duration::from_secs(1)),
        })
    }

    pub fn update(&mut self, dt: Duration, input_state: &InputState) {
        self.fps.record(1.0 / dt.as_secs_f32());

        {
            profile!("recv");

            // At this point, we have always sent one input to the update thread,
            // so we can wait here until we receive the output.
            self.last_output = Some(self.update.recv_output());
        }

        {
            profile!("send");

            // Submit the next input for the update thread. Updating can then run
            // at the same time as drawing the previous output.
            let window_events = std::mem::replace(&mut self.next_window_events, Vec::new());

            let input = update::Input {
                dt,
                window_events,
                input_state: input_state.clone(),
                target_size: self.target_size,
            };

            self.update.send_input(input);
        }
    }

    pub fn draw<F: glium::backend::Facade, S: glium::Surface>(
        &mut self,
        facade: &F,
        target: &mut S,
    ) -> Result<(), rendology::DrawError> {
        self.target_size = target.get_dimensions();

        if let Some(output) = self.last_output.take() {
            let input = draw::Input {
                recreate_pipeline: None,
                stage: &output.render_stage,
                context: output.render_context.clone(),
            };
            self.draw.draw(facade, &input, target)?;
        }

        Ok(())
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
