use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use coarse_prof::profile;
use glium::glutin;
use log::{info, warn};
use nalgebra as na;
use rendology::Camera;

use crate::config::{Config, ViewConfig};
use crate::edit::{editor, Editor};
use crate::edit_camera_view::{EditCameraView, EditCameraViewInput};
use crate::exec::TickTime;
use crate::input_state::InputState;
use crate::render;

pub struct Input {
    pub dt: Duration,
    pub target_size: (u32, u32),
    pub input_state: InputState,
    pub window_events: Vec<(InputState, glutin::WindowEvent)>,
    pub editor_ui_output: editor::ui::Output,
}

pub struct Output {
    pub render_stage: render::Stage,
    pub render_context: render::Context,
    pub editor_ui_input: Option<editor::ui::Input>,
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
                                // The corresponding sender has disconnected. Shut down
                                // gracefully. This should not happen in practice, since
                                // the sender sends `Command::Terminate`.
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
    fov: f32,
    camera: Camera,
    edit_camera_view: EditCameraView,
    edit_camera_view_input: EditCameraViewInput,

    editor: Editor,
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

        Self {
            fov,
            camera,
            edit_camera_view,
            edit_camera_view_input,
            editor,
        }
    }

    pub fn update(&mut self, input: Input) -> Output {
        let viewport_size =
            na::Vector2::new(input.target_size.0 as f32, input.target_size.1 as f32);
        self.camera.viewport_size = viewport_size;
        self.camera.projection = perspective_matrix(self.fov, &viewport_size);

        for (input_state, window_event) in input.window_events.into_iter() {
            self.edit_camera_view_input.on_event(&window_event);
            self.editor.on_event(&input_state, &window_event);

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

        self.editor.on_ui_output(&input.editor_ui_output);

        self.editor.update(
            input.dt,
            &input.input_state,
            &self.camera,
            &mut self.edit_camera_view,
        );

        self.edit_camera_view_input.update(
            input.dt.as_secs_f32(),
            &input.input_state,
            &mut self.edit_camera_view,
        );
        self.camera.view = self.edit_camera_view.view();

        self.render()
    }

    fn render(&mut self) -> Output {
        profile!("render");

        let mut render_stage = render::Stage::default();
        self.editor.render(&mut render_stage);

        let main_light_pos = na::Point3::new(
            15.0 + 20.0 * (std::f32::consts::PI / 4.0).cos(),
            15.0 + 20.0 * (std::f32::consts::PI / 4.0).sin(),
            20.0,
        );

        render_stage.lights.push(rendology::Light {
            position: main_light_pos,
            attenuation: na::Vector3::new(1.0, 0.0, 0.0),
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
            tick_time: TickTime::zero(),
        };

        let editor_ui_input = self.editor.ui_input();

        Output {
            render_stage,
            render_context,
            editor_ui_input: Some(editor_ui_input),
        }
    }
}

fn perspective_matrix(fov_radians: f32, viewport_size: &na::Vector2<f32>) -> na::Matrix4<f32> {
    let projection =
        na::Perspective3::new(viewport_size.x / viewport_size.y, fov_radians, 0.1, 10000.0);
    projection.to_homogeneous()
}
