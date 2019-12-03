mod ui;

use std::time::Duration;

use floating_duration::TimeAsFloat;
use log::info;
use nalgebra as na;
use coarse_prof::profile;

use glium::glutin;

use crate::config::{self, Config};
use crate::edit::Editor;
use crate::edit_camera_view::{EditCameraView, EditCameraViewInput};
use crate::exec::level_progress::InputsOutputsProgress;
use crate::exec::play::{self, Play};
use crate::exec::{ExecView, LevelStatus};
use crate::input_state::InputState;
use crate::machine::{level, Machine};
use crate::render::{self, resources, Camera, Light, RenderLists, Resources};
use crate::util::stats;

pub struct Game {
    config: Config,

    camera: Camera,
    edit_camera_view: EditCameraView,
    edit_camera_view_input: EditCameraViewInput,

    resources: Resources,
    render_pipeline: render::Pipeline,
    render_lists: RenderLists,

    editor: Editor,
    play: Play,
    exec: Option<(play::Status, ExecView)>,

    /// Current example to show for the level inputs/outputs. Optionally, store
    /// the progress through the inputs/outputs when executing.
    inputs_outputs_example: Option<(level::InputsOutputs, Option<InputsOutputsProgress>)>,

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

        let resources = Resources::create(facade).map_err(CreationError::RenderResources)?;
        let render_pipeline =
            render::Pipeline::create(facade, &config.render_pipeline, &config.view)
                .map_err(CreationError::RenderPipeline)?;
        let render_lists = RenderLists::default();

        let editor = Editor::new(&config.editor, initial_machine);
        let play = Play::new(&config.play);

        let inputs_outputs_example = editor
            .machine()
            .level
            .as_ref()
            .map(|level| (level.spec.gen_inputs_outputs(&mut rand::thread_rng()), None));

        Ok(Game {
            config: config.clone(),
            camera,
            edit_camera_view,
            edit_camera_view_input,
            resources,
            render_pipeline,
            render_lists,
            editor,
            play,
            exec: None,
            inputs_outputs_example,
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

            self.render_pipeline =
                render::Pipeline::create(facade, &self.config.render_pipeline, &self.config.view)
                    .map_err(CreationError::RenderPipeline)?;

            self.recreate_render_pipeline = false;
        }

        Ok(())
    }

    pub fn draw<S: glium::Surface>(
        &mut self,
        display: &glium::backend::glutin::Display,
        target: &mut S,
    ) -> Result<(), render::DrawError> {
        {
            profile!("render");

            self.render_lists.clear();

            if let Some((play_status, exec)) = self.exec.as_mut() {
                exec.render(&play_status.time(), &mut self.render_lists);
            } else {
                self.editor.render(&mut self.render_lists)?;
            }
        };

        let render_context = render::Context {
            camera: self.camera.clone(),
            elapsed_time_secs: self.elapsed_time.as_fractional_secs() as f32,
            tick_progress: self
                .exec
                .as_ref()
                .map_or(0.0, |(play_status, _)| play_status.tick_progress()),
            main_light_pos: na::Point3::new(
                15.0 + 20.0 * (std::f32::consts::PI / 4.0).cos(),
                15.0 + 20.0 * (std::f32::consts::PI / 4.0).sin(),
                20.0,
            ),
            main_light_center: na::Point3::new(15.0, 15.0, 0.0),
        };

        self.render_lists.lights.push(Light {
            position: render_context.main_light_pos,
            attenuation: na::Vector3::new(1.0, 0.0, 0.0),
            color: na::Vector3::new(1.0, 1.0, 1.0),
            is_main: true,
            ..Default::default()
        });

        self.render_pipeline.draw_frame(
            display,
            &self.resources,
            &render_context,
            &self.render_lists,
            target,
        )?;

        // Render screen-space stuff on top
        profile!("ortho");

        let ortho_projection = na::Matrix4::new_orthographic(
            0.0,
            self.camera.viewport.z,
            self.camera.viewport.w,
            0.0,
            -10.0,
            10.0,
        );
        let ortho_camera = Camera {
            projection: ortho_projection,
            view: na::Matrix4::identity(),
            ..self.camera.clone()
        };
        let ortho_render_context = render::Context {
            camera: ortho_camera,
            ..render_context
        };
        let ortho_parameters = glium::DrawParameters {
            blend: glium::draw_parameters::Blend::alpha_blending(),
            ..Default::default()
        };
        self.render_lists.ortho.draw(
            &self.resources,
            &ortho_render_context,
            &self.resources.plain_program,
            &ortho_parameters,
            target,
        )?;

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
            }
            (true, Some(play_status)) => {
                // Advance execution
                if let Some((s, exec)) = self.exec.as_mut() {
                    *s = play_status;

                    if let play::Status::Playing {
                        num_ticks_since_last_update,
                        ref mut time,
                        ..
                    } = s
                    {
                        for _ in 0..*num_ticks_since_last_update {
                            exec.run_tick();

                            if exec.level_status() != LevelStatus::Running {
                                *s = play::Status::Finished { time: time.clone() };
                                break;
                            }
                        }
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
        facade: &F,
        new_window_size: glutin::dpi::LogicalSize,
    ) -> Result<(), CreationError> {
        self.config.view.window_size = new_window_size;

        self.camera.projection = Self::perspective_matrix(&self.config.view, new_window_size);
        self.camera.viewport = na::Vector4::new(
            0.0,
            0.0,
            new_window_size.width as f32,
            new_window_size.height as f32,
        );

        self.render_pipeline
            .on_window_resize(facade, new_window_size)
            .map_err(CreationError::RenderPipeline)?;

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
    RenderPipeline(render::pipeline::CreationError),
    RenderResources(resources::CreationError),
}
