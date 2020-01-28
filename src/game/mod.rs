mod ui;

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

pub struct Game {
    config: Config,

    camera: Camera,
    edit_camera_view: EditCameraView,
    edit_camera_view_input: EditCameraViewInput,

    render_pipeline: render::Pipeline,
    render_stage: render::Stage,

    editor: Editor,
    play: Play,
    exec: Option<(play::Status, ExecView)>,

    /// Current example to show for the level inputs/outputs.
    level_example: Option<LevelProgress>,

    elapsed_time: Duration,
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
    ) -> Result<Game, CreationError> {
        info!("Creating resources");

        let viewport_size = na::Vector2::new(
            config.view.window_size.width as f32,
            config.view.window_size.height as f32,
        );
        let camera = Camera::new(
            viewport_size,
            Self::perspective_matrix(&config.view, config.view.window_size),
        );
        let edit_camera_view = EditCameraView::new();
        let edit_camera_view_input = EditCameraViewInput::new(&config.camera);

        let render_pipeline = render::Pipeline::create(
            facade,
            &config.render_pipeline,
            config.view.window_size.into(),
        )
        .map_err(CreationError::RenderPipeline)?;
        let render_stage = render::Stage::default();

        let level_example = initial_machine.level.as_ref().map(|level| {
            let inputs_outputs = level.spec.gen_inputs_outputs(&mut rand::thread_rng());
            LevelProgress::new(None, inputs_outputs)
        });

        let editor = Editor::new(&config.editor, initial_machine);
        let play = Play::new(&config.play);

        Ok(Game {
            config: config.clone(),
            camera,
            edit_camera_view,
            edit_camera_view_input,
            render_pipeline,
            render_stage,
            editor,
            play,
            exec: None,
            level_example,
            elapsed_time: Default::default(),
            fps: stats::Variable::new(Duration::from_secs(1)),
            show_config_ui: false,
            show_debug_ui: false,
            recreate_render_pipeline: false,
        })
    }

    pub fn update_resources<F: glium::backend::Facade>(
        &mut self,
        facade: &F,
    ) -> Result<(), CreationError> {
        if self.recreate_render_pipeline {
            info!(
                "Recreating render pipeline with config: {:?}",
                self.config.render_pipeline
            );

            self.render_pipeline = render::Pipeline::create(
                facade,
                &self.config.render_pipeline,
                self.config.view.window_size.into(),
            )
            .map_err(CreationError::RenderPipeline)?;

            self.recreate_render_pipeline = false;
        }

        Ok(())
    }

    pub fn draw<S: glium::Surface>(
        &mut self,
        display: &glium::backend::glutin::Display,
        target: &mut S,
    ) -> Result<(), rendology::DrawError> {
        self.camera.viewport_size.x = target.get_dimensions().0 as f32;
        self.camera.viewport_size.y = target.get_dimensions().1 as f32;

        {
            profile!("render");

            if let Some((play_status, exec)) = self.exec.as_mut() {
                exec.render(&play_status.time(), &mut self.render_stage);
            } else {
                self.editor.render(&mut self.render_stage)?;
            }
        };

        let render_context = render::Context {
            rendology: rendology::Context {
                camera: self.camera.clone(),
                main_light_pos: na::Point3::new(
                    15.0 + 20.0 * (std::f32::consts::PI / 4.0).cos(),
                    15.0 + 20.0 * (std::f32::consts::PI / 4.0).sin(),
                    20.0,
                ),
                main_light_center: na::Point3::new(15.0, 15.0, 0.0),
                ambient_light: na::Vector3::new(0.3, 0.3, 0.3),
            },
            tick_progress: self
                .exec
                .as_ref()
                .map_or(0.0, |(play_status, _)| play_status.tick_progress()),
        };

        self.render_stage.lights.push(Light {
            position: render_context.rendology.main_light_pos,
            attenuation: na::Vector3::new(1.0, 0.0, 0.0),
            color: na::Vector3::new(1.0, 1.0, 1.0),
            is_main: true,
            ..Default::default()
        });

        let time = self
            .exec
            .as_ref()
            .map_or(0.0, |(play_status, _)| play_status.time().to_f32());
        self.render_pipeline.draw_frame(
            display,
            &render_context,
            time,
            &self.render_stage,
            target,
        )?;

        self.render_stage.clear();

        Ok(())
    }

    pub fn update(&mut self, dt: Duration, input_state: &InputState) {
        self.elapsed_time += dt;
        let dt_secs = dt.as_fractional_secs() as f32;

        self.fps.record(1.0 / dt_secs);

        // Update play status
        let play_status = self
            .play
            .update_status(dt, self.exec.as_ref().map(|(play_status, _)| play_status));

        match (self.exec.is_some(), play_status) {
            (false, Some(play_status @ play::Status::Playing { .. })) => {
                // Start execution
                let exec = ExecView::new(&self.config.exec, self.editor.machine().clone());
                self.exec = Some((play_status, exec));
            }
            (true, None) => {
                // Stop execution
                self.exec = None;

                self.render_pipeline.clear_particles();
            }
            (true, Some(play_status)) => {
                // Advance execution
                if let Some((s, exec)) = self.exec.as_mut() {
                    *s = play_status;

                    if let play::Status::Playing {
                        num_ticks_since_last_update,
                        prev_time,
                        time,
                        ..
                    } = s.clone()
                    {
                        let mut last_transduce_time = prev_time.clone();

                        if num_ticks_since_last_update > 0 {
                            // Finish off transducing the previous tick.
                            if let Some(prev_time) = prev_time.as_ref() {
                                let mut end_of_last_tick = prev_time.clone();
                                end_of_last_tick.next_tick_timer.set_progress(1.0);

                                exec.transduce(
                                    prev_time,
                                    &end_of_last_tick,
                                    &mut self.render_stage,
                                );
                                last_transduce_time = Some(end_of_last_tick);
                            }
                        }

                        for _ in 0..num_ticks_since_last_update {
                            exec.run_tick();

                            if exec.next_level_status() != LevelStatus::Running {
                                *s = play::Status::Finished { time: time.clone() };
                                break;
                            }
                        }

                        let last_transduce_time = last_transduce_time.unwrap_or(TickTime::zero());

                        exec.transduce(&last_transduce_time, &time, &mut self.render_stage);
                    }
                }
            }
            _ => (),
        }

        if let Some((_, exec)) = self.exec.as_mut() {
            exec.update(dt, input_state, &self.camera, &self.edit_camera_view);
        } else {
            self.editor
                .update(dt, input_state, &self.camera, &mut self.edit_camera_view);
        }

        self.edit_camera_view_input
            .update(dt_secs, input_state, &mut self.edit_camera_view);
        self.camera.view = self.edit_camera_view.view();
    }

    pub fn on_event(&mut self, input_state: &InputState, event: &glutin::WindowEvent) {
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

        self.edit_camera_view_input.on_event(event);
        self.play.on_event(event);

        if let Some((_, exec_view)) = self.exec.as_mut() {
            exec_view.on_event(event);
        } else {
            self.editor.on_event(input_state, event);
        }
    }

    pub fn on_window_resize<F: glium::backend::Facade>(
        &mut self,
        _facade: &F,
        new_window_size: glutin::dpi::LogicalSize,
    ) -> Result<(), CreationError> {
        self.config.view.window_size = new_window_size;

        self.camera.projection = Self::perspective_matrix(&self.config.view, new_window_size);

        Ok(())
    }

    fn perspective_matrix(
        config: &config::ViewConfig,
        window_size: glutin::dpi::LogicalSize,
    ) -> na::Matrix4<f32> {
        let projection = na::Perspective3::new(
            window_size.width as f32 / window_size.height as f32,
            config.fov_degrees.to_radians() as f32,
            0.1,
            10000.0,
        );
        projection.to_homogeneous()
    }
}

#[derive(Debug)]
pub enum CreationError {
    RenderPipeline(rendology::pipeline::CreationError),
}
