mod draw;
mod ui;
mod update;

use std::time::Duration;

use coarse_prof::profile;
use log::info;

use glium::glutin;

use crate::config::Config;
use crate::edit::Editor;
use crate::exec::play::{self, Play};
use crate::exec::LevelStatus;
use crate::input_state::InputState;
use crate::machine::Machine;
use crate::util::stats;

use draw::Draw;
use update::{Update, UpdateRunner};

pub struct Game {
    config: Config,

    update: UpdateRunner,
    draw: Draw,

    target_size: (u32, u32),

    last_output: Option<update::Output>,
    next_input_stage: update::InputStage,

    play: Play,
    play_status: Option<play::Status>,

    fps: stats::Variable,
    show_config_ui: bool,
    show_debug_ui: bool,
    recreate_render_pipeline: bool,
}

impl Game {
    pub fn create<F: glium::backend::Facade>(
        facade: &F,
        config: &Config,
        initial_machine: Machine,
    ) -> Result<Game, rendology::pipeline::CreationError> {
        info!("Creating resources");

        let editor = Editor::new(&config.editor, initial_machine);
        let mut update = UpdateRunner::spawn(Update::new_editor(config, editor));
        let draw = Draw::create(facade, config)?;

        // TODO: Account for DPI in initialization
        let target_size = config.view.window_size.into();

        // Kick off the update loop, so that we get our first `update::Output`
        // to draw.
        update.send_input(update::InputStage::default().into_input(
            Duration::from_secs(0),
            target_size,
            InputState::empty(1.0),
            None,
        ));

        let play = Play::new(&config.play);

        Ok(Game {
            config: config.clone(),
            update,
            draw,
            target_size,
            last_output: None,
            next_input_stage: update::InputStage::default(),
            play,
            play_status: None,
            fps: stats::Variable::new(Duration::from_secs(1)),
            show_config_ui: false,
            show_debug_ui: false,
            recreate_render_pipeline: false,
        })
    }

    pub fn update(&mut self, dt: Duration, input_state: &InputState) {
        self.fps.record(1.0 / dt.as_secs_f32());

        {
            profile!("recv");

            // At this point, we have always sent one input to the update thread,
            // so we can wait here until we receive the output.
            let output = self.update.recv_output();

            // If execution has ended (due to the level being failed or
            // completed), update the play status.
            if output.next_level_status != Some(LevelStatus::Running) {
                self.play_status = match self.play_status.clone() {
                    Some(play::Status::Playing { time, .. }) => {
                        Some(play::Status::Finished { time })
                    }
                    x => x,
                }
            }

            self.last_output = Some(output);
        }

        // Note that play status may be set to `Finished` above in this
        // function, so order is important here.
        let old_play_status = self.play_status.clone();
        self.play_status = self.play.update_status(dt, self.play_status.as_ref());

        // Did we just stop execution?
        if old_play_status.is_some() && self.play_status.is_none() {
            self.draw.clean_up_after_exec();
        }

        {
            profile!("send");

            // Submit the next input for the update thread. Updating can then run
            // at the same time as drawing the previous output.
            let next_input_stage =
                std::mem::replace(&mut self.next_input_stage, Default::default());
            let next_input = next_input_stage.into_input(
                dt,
                self.target_size,
                input_state.clone(),
                self.play_status.clone(),
            );

            self.update.send_input(next_input);
        }
    }

    pub fn create_resources<F: glium::backend::Facade>(
        &mut self,
        facade: &F,
    ) -> Result<(), rendology::pipeline::CreationError> {
        if self.recreate_render_pipeline {
            info!(
                "Recreating render pipeline with config: {:?}",
                self.config.render_pipeline,
            );

            self.recreate_render_pipeline = false;

            self.draw = Draw::create(facade, &self.config)?;
        }

        Ok(())
    }

    pub fn draw<F: glium::backend::Facade, S: glium::Surface>(
        &mut self,
        facade: &F,
        target: &mut S,
    ) -> Result<(), rendology::DrawError> {
        self.target_size = target.get_dimensions();

        if let Some(output) = self.last_output.take() {
            let input = draw::Input {
                stage: &output.render_stage,
                context: output.render_context.clone(),
            };
            self.draw.draw(facade, &input, target)?;
        }

        Ok(())
    }

    pub fn on_event(&mut self, input_state: &InputState, event: &glutin::WindowEvent) {
        self.next_input_stage
            .window_events
            .push((input_state.clone(), event.clone()));

        self.play.on_event(event);

        // Some shortcuts for debugging
        if let glutin::WindowEvent::KeyboardInput { input, .. } = event {
            if input.state == glutin::ElementState::Pressed
                && input.virtual_keycode == Some(glutin::VirtualKeyCode::F5)
            {
                self.show_config_ui = !self.show_config_ui;
            } else if input.state == glutin::ElementState::Pressed
                && input.virtual_keycode == Some(glutin::VirtualKeyCode::F6)
            {
                self.show_debug_ui = !self.show_debug_ui;
            }
        }
    }

    pub fn on_window_resize<F: glium::backend::Facade>(
        &mut self,
        _facade: &F,
        _new_window_size: glutin::dpi::LogicalSize,
    ) -> () {
        ()
    }
}
