use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use coarse_prof::profile;
use glium::glutin;
use log::{info, warn};
use nalgebra as na;
use rendology::Camera;

use crate::config::Config;
use crate::edit::{editor, Editor};
use crate::edit_camera_view::{EditCameraView, EditCameraViewInput};
use crate::exec::{play, ExecView, LevelProgress, LevelStatus, TickTime};
use crate::input_state::InputState;
use crate::machine::Level;
use crate::render;

#[derive(Debug, Clone, Default)]
pub struct InputStage {
    pub window_events: Vec<(InputState, glutin::WindowEvent)>,
    pub editor_ui_output: editor::ui::Output,
    pub generate_level_example: bool,
}

impl InputStage {
    pub fn into_input(
        self,
        dt: Duration,
        target_size: (u32, u32),
        input_state: InputState,
        play_status: Option<play::Status>,
    ) -> Input {
        Input {
            dt,
            target_size,
            input_state,
            play_status,
            stage: self,
        }
    }
}

pub struct Input {
    pub dt: Duration,
    pub target_size: (u32, u32),
    pub input_state: InputState,
    pub play_status: Option<play::Status>,
    pub stage: InputStage,
}

pub struct Output {
    pub render_stage: render::Stage,
    pub render_context: render::Context,
    pub editor_ui_input: Option<editor::ui::Input>,
    pub level_progress: Option<(Level, LevelProgress)>,
    pub next_level_status: Option<LevelStatus>,
}

enum Command {
    Terminate,
    Run(Input),
}

pub struct UpdateRunner {
    command_send: mpsc::Sender<Command>,
    output_recv: mpsc::Receiver<Output>,
    thread: Option<thread::JoinHandle<()>>,
}

impl UpdateRunner {
    pub fn spawn(update: Update) -> Self {
        let (command_send, command_recv) = mpsc::channel();
        let (output_send, output_recv) = mpsc::channel();

        let thread = thread::spawn(move || {
            Self::run(update, command_recv, output_send);
        });

        UpdateRunner {
            command_send,
            output_recv,
            thread: Some(thread),
        }
    }

    pub fn send_input(&mut self, input: Input) {
        // It makes sense to unwrap here, since err means that the update
        // thread shut down for some unintended reason.
        self.command_send.send(Command::Run(input)).unwrap();
    }

    pub fn recv_output(&mut self) -> Output {
        // It makes sense to unwrap here, since err means that the update
        // thread shut down for some unintended reason.
        self.output_recv.recv().unwrap()
    }

    fn run(
        mut update: Update,
        command_recv: mpsc::Receiver<Command>,
        output_send: mpsc::Sender<Output>,
    ) {
        loop {
            profile!("update_thread");

            let command = {
                profile!("recv");
                command_recv.recv()
            };

            if let Ok(command) = command {
                match command {
                    Command::Terminate => {
                        info!("Received termination command, shutting down update thread");
                        return;
                    }
                    Command::Run(input) => {
                        let output = {
                            profile!("run");
                            update.update(input)
                        };
                        {
                            profile!("send");
                            let result = output_send.send(output);

                            if result.is_err() {
                                // The corresponding sender has disconnected.
                                // Shut down gracefully. This should not happen
                                // in practice, since the sender sends
                                // `Command::Terminate`.
                                warn!("Sender disconnected, shutting down update thread");
                                return;
                            }
                        }
                    }
                }
            } else {
                // The corresponding sender has disconnected. Shut down
                // gracefully. This should not happen in practice, since
                // the sender sends `Command::Terminate`.
                warn!("Sender disconnected, shutting down update thread");
                return;
            }
        }
    }
}

impl Drop for UpdateRunner {
    fn drop(&mut self) {
        info!("Shutting down update thread");

        let result = self.command_send.send(Command::Terminate);

        if result.is_err() {
            // The update thread has disconnected already. This should not
            // happen in practice. We ignore the error here, so that we can see
            // the panic when joining below.
            warn!("Update thread disconnected, ignoring");
            return;
        }

        // Forward panics of the update thread to the outer thread.
        self.thread.take().unwrap().join().unwrap();
    }
}

pub struct Update {
    config: Config,

    fov: f32,
    camera: Camera,
    edit_camera_view: EditCameraView,
    edit_camera_view_input: EditCameraViewInput,

    editor: Editor,
    exec_view: Option<ExecView>,

    /// Current input/output example to show for the level.
    level_progress: Option<LevelProgress>,
}

impl Update {
    pub fn new_editor(config: &Config, editor: Editor) -> Self {
        let fov = config.view.fov_degrees.to_radians() as f32;

        // TODO: Account for DPI in initialization
        let viewport_size = na::Vector2::new(
            config.view.window_size.width as f32,
            config.view.window_size.height as f32,
        );
        let camera = Camera::new(viewport_size, perspective_matrix(fov, &viewport_size));
        let edit_camera_view = EditCameraView::new();
        let edit_camera_view_input = EditCameraViewInput::new(&config.camera);

        let level_progress = editor.machine().level.as_ref().map(|level| {
            let inputs_outputs = level.spec.gen_inputs_outputs(&mut rand::thread_rng());
            LevelProgress::new(None, inputs_outputs)
        });

        Self {
            config: config.clone(),
            fov,
            camera,
            edit_camera_view,
            edit_camera_view_input,
            editor,
            exec_view: None,
            level_progress,
        }
    }

    pub fn update(&mut self, input: Input) -> Output {
        let mut render_stage = render::Stage::default();
        self.sync_with_play_status(input.play_status.as_ref(), &mut render_stage);

        let viewport_size =
            na::Vector2::new(input.target_size.0 as f32, input.target_size.1 as f32);
        self.camera.viewport_size = viewport_size;
        self.camera.projection = perspective_matrix(self.fov, &viewport_size);

        for (_, window_event) in input.stage.window_events.iter() {
            self.edit_camera_view_input.on_event(window_event);

            // Print thread-local profiling:
            if let glutin::WindowEvent::KeyboardInput { input, .. } = window_event {
                if input.state == glutin::ElementState::Pressed {
                    match input.virtual_keycode {
                        Some(glutin::VirtualKeyCode::P) => {
                            coarse_prof::write(&mut std::io::stdout()).unwrap();
                            coarse_prof::reset();
                        }
                        _ => {}
                    }
                }
            }
        }

        if let Some(exec_view) = self.exec_view.as_mut() {
            // Execution mode

            for (_, window_event) in input.stage.window_events.iter() {
                exec_view.on_event(window_event);
            }

            exec_view.update(
                input.dt,
                &input.input_state,
                &self.camera,
                &self.edit_camera_view,
            );

            self.level_progress = exec_view.level_progress().cloned();
        } else {
            // Editor mode

            for (input_state, window_event) in input.stage.window_events.iter() {
                self.editor.on_event(input_state, window_event);
            }

            self.editor.on_ui_output(&input.stage.editor_ui_output);
            self.editor.update(
                input.dt,
                &input.input_state,
                &self.camera,
                &mut self.edit_camera_view,
            );

            if input.stage.generate_level_example {
                self.level_progress = self.editor.machine().level.as_ref().map(|level| {
                    let inputs_outputs = level.spec.gen_inputs_outputs(&mut rand::thread_rng());
                    LevelProgress::new(None, inputs_outputs)
                });
            }
        }

        self.edit_camera_view_input.update(
            input.dt.as_secs_f32(),
            &input.input_state,
            &mut self.edit_camera_view,
        );
        self.camera.view = self.edit_camera_view.view();

        self.render(input, render_stage)
    }

    pub fn sync_with_play_status(
        &mut self,
        play_status: Option<&play::Status>,
        render_stage: &mut render::Stage,
    ) {
        // Do we need to start/stop execution?
        if self.exec_view.is_some() != play_status.is_some() {
            if play_status.is_some() {
                // Start execution
                self.exec_view = Some(ExecView::new(
                    &self.config.exec,
                    self.editor.machine().clone(),
                ));
            } else {
                // Stop execution
                self.exec_view = None;
            }
        }

        assert!(self.exec_view.is_some() == play_status.is_some());

        // Advance execution?
        if let Some(play::Status::Playing {
            num_ticks_since_last_update,
            prev_time,
            time,
            ..
        }) = play_status
        {
            // Safe to unwrap here, since we have synchronized execution status
            // above.
            let exec_view = self.exec_view.as_mut().unwrap();
            let mut last_transduce_time = prev_time.clone();

            if *num_ticks_since_last_update > 1 {
                // Finish off transducing the previous tick.
                if let Some(prev_time) = prev_time.as_ref() {
                    let mut end_of_last_tick = prev_time.clone();
                    end_of_last_tick.next_tick_timer.set_progress(1.0);

                    // Ignore these events when speeding through the simulation,
                    // preventing massive slowdowns.
                    if *num_ticks_since_last_update == 1 {
                        exec_view.transduce(
                            prev_time,
                            &end_of_last_tick,
                            &self.edit_camera_view.eye(),
                            render_stage,
                        );
                    }

                    last_transduce_time = Some(end_of_last_tick);
                }
            }

            for _ in 0..*num_ticks_since_last_update {
                exec_view.run_tick();

                if exec_view.next_level_status() != LevelStatus::Running {
                    break;
                }
            }

            let last_transduce_time = last_transduce_time.unwrap_or_else(TickTime::zero);
            exec_view.transduce(
                &last_transduce_time,
                &time,
                &self.edit_camera_view.eye(),
                render_stage,
            );
        }
    }

    fn render(&mut self, input: Input, mut render_stage: render::Stage) -> Output {
        profile!("render");

        if let Some(exec_view) = self.exec_view.as_mut() {
            // Safe to unwrap here, since we have synchronized execution status
            // above.
            let tick_time = input.play_status.as_ref().unwrap().time();

            exec_view.render(tick_time, &mut render_stage);
        } else {
            self.editor.render(&mut render_stage);
        }

        let main_light_pos = na::Point3::new(
            15.0 + 20.0 * (std::f32::consts::PI / 4.0).cos(),
            15.0 + 20.0 * (std::f32::consts::PI / 4.0).sin(),
            20.0,
        );

        render_stage.lights.push(rendology::Light {
            position: main_light_pos,
            attenuation: na::Vector4::new(1.0, 0.0, 0.0, 0.0),
            color: na::Vector3::new(1.0, 1.0, 1.0),
            is_main: true,
            ..Default::default()
        });

        let render_context = render::Context {
            rendology: rendology::Context {
                camera: self.camera.clone(),
                main_light_pos,
                main_light_center: na::Point3::new(15.0, 15.0, 0.0),
                ambient_light: na::Vector3::new(0.3, 0.3, 0.3),
            },
            tick_time: input
                .play_status
                .map_or_else(TickTime::zero, |status| status.time().clone()),
        };

        let editor_ui_input = if self.exec_view.is_none() {
            Some(self.editor.ui_input())
        } else {
            None
        };

        let level_progress = self
            .editor
            .machine()
            .level
            .clone()
            .and_then(|level| self.level_progress.clone().map(|example| (level, example)));

        let next_level_status = self
            .exec_view
            .as_ref()
            .map(|exec_view| exec_view.next_level_status());

        Output {
            render_stage,
            render_context,
            editor_ui_input,
            level_progress,
            next_level_status,
        }
    }
}

fn perspective_matrix(fov_radians: f32, viewport_size: &na::Vector2<f32>) -> na::Matrix4<f32> {
    let projection =
        na::Perspective3::new(viewport_size.x / viewport_size.y, fov_radians, 0.1, 10000.0);
    projection.to_homogeneous()
}
