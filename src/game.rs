use std::time::Duration;

use floating_duration::TimeAsFloat;
use imgui::{im_str, ImString};
use log::info;
use nalgebra as na;

use glium::glutin;

use crate::config::{self, Config};
use crate::edit::Editor;
use crate::exec::play::{self, Play};
use crate::exec::{Exec, ExecView, LevelStatus};
use crate::input_state::InputState;
use crate::machine::{self, level, Block, Machine};
use crate::util::stats;

use crate::render::camera::{Camera, EditCameraView, EditCameraViewInput};
use crate::render::{self, fxaa, resources, Light, RenderLists, Resources};

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
            &mut self.render_lists,
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
                self.exec.as_mut().map(|(s, exec)| {
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
                });
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

    pub fn ui(&mut self, ui: &imgui::Ui) {
        let window_size = na::Vector2::new(self.camera.viewport.z, self.camera.viewport.w);

        if let Some((_, exec_view)) = self.exec.as_mut() {
            exec_view.ui(ui);
        } else {
            self.editor.ui(ui);
        }

        let play_state = self.exec.as_ref().map(|(play_state, _)| play_state);
        self.play.ui(window_size, play_state, ui);

        if self.show_config_ui {
            imgui::Window::new(im_str!("Config"))
                .horizontal_scrollbar(true)
                .position([window_size.x, 10.0], imgui::Condition::FirstUseEver)
                .position_pivot([1.0, 0.0])
                .always_auto_resize(true)
                .bg_alpha(0.8)
                .build(&ui, || {
                    let mut shadow_mapping = self.config.render_pipeline.shadow_mapping.is_some();
                    if ui.checkbox(im_str!("Shadow mapping"), &mut shadow_mapping) {
                        self.config.render_pipeline.shadow_mapping = if shadow_mapping {
                            Some(Default::default())
                        } else {
                            None
                        };
                    }

                    let mut deferred_shading =
                        self.config.render_pipeline.deferred_shading.is_some();
                    if ui.checkbox(im_str!("Deferred shading"), &mut deferred_shading) {
                        self.config.render_pipeline.deferred_shading = if deferred_shading {
                            Some(Default::default())
                        } else {
                            None
                        };
                    }

                    let mut glow = self.config.render_pipeline.glow.is_some();
                    if ui.checkbox(im_str!("Glow"), &mut glow) {
                        self.config.render_pipeline.glow =
                            if glow { Some(Default::default()) } else { None };
                    }

                    let mut gamma = self.config.render_pipeline.gamma_correction.unwrap_or(1.0);

                    imgui::Slider::new(im_str!("Gamma"), 0.3..=4.0).build(ui, &mut gamma);

                    self.config.render_pipeline.gamma_correction = Some(gamma);

                    let mut hdr = self.config.render_pipeline.hdr.is_some();
                    if ui.checkbox(im_str!("HDR"), &mut hdr) {
                        self.config.render_pipeline.hdr = if hdr { Some(42.0) } else { None };
                    }

                    ui.separator();

                    let mut fxaa_quality = self
                        .config
                        .render_pipeline
                        .fxaa
                        .as_ref()
                        .map(|config| config.quality);
                    ui.radio_button(im_str!("No anti-aliasing"), &mut fxaa_quality, None);
                    ui.radio_button(
                        im_str!("FXAA (low)"),
                        &mut fxaa_quality,
                        Some(fxaa::Quality::Low),
                    );
                    ui.radio_button(
                        im_str!("FXAA (medium)"),
                        &mut fxaa_quality,
                        Some(fxaa::Quality::Medium),
                    );
                    ui.radio_button(
                        im_str!("FXAA (high)"),
                        &mut fxaa_quality,
                        Some(fxaa::Quality::High),
                    );

                    self.config.render_pipeline.fxaa =
                        fxaa_quality.map(|quality| fxaa::Config { quality });

                    ui.separator();

                    if ui.button(im_str!("Apply"), [80.0, 20.0]) {
                        self.recreate_render_pipeline = true;
                    }
                });
        }

        if self.show_debug_ui {
            imgui::Window::new(im_str!("Debug"))
                .horizontal_scrollbar(true)
                .position([window_size.x, 300.0], imgui::Condition::FirstUseEver)
                .position_pivot([1.0, 0.0])
                .always_auto_resize(true)
                .bg_alpha(0.8)
                .build(&ui, || {
                    ui.text(&ImString::new(format!(
                        "FPS: {:.1}",
                        self.fps.recent_average()
                    )));
                });
        }

        if let Some(level) = self.editor.machine().level.as_ref() {
            if let Some((_, exec)) = self.exec.as_ref() {
                // During execution, set the shown example to the generated
                // one. Also remember the progress, so that it can still be
                // shown after execution.
                if let Some(example) = exec.inputs_outputs() {
                    self.inputs_outputs_example = Some((
                        example.clone(),
                        Some(InputsOutputsProgress::new_from_exec(example, exec.exec())),
                    ));
                }
            }

            // UI allows generating new example when not executing
            let mut updated_example = None;

            imgui::Window::new(im_str!("Level"))
                .horizontal_scrollbar(true)
                .position([window_size.x / 2.0, 10.0], imgui::Condition::FirstUseEver)
                .position_pivot([0.5, 0.0])
                .always_auto_resize(true)
                .bg_alpha(0.8)
                .build(&ui, || {
                    let goal = "Goal: ".to_string() + &level.spec.description();
                    ui.bullet_text(&ImString::new(&goal));

                    let status = "Status: ".to_string()
                        + &if let Some((_, exec)) = self.exec.as_ref() {
                            match exec.level_status() {
                                LevelStatus::Running => "Running".to_string(),
                                LevelStatus::Completed => "Completed!".to_string(),
                                LevelStatus::Failed => "Failed".to_string(),
                            }
                        } else {
                            "Editing".to_string()
                        };

                    ui.bullet_text(&ImString::new(&status));

                    imgui::TreeNode::new(ui, im_str!("Show example"))
                        .opened(false, imgui::Condition::FirstUseEver)
                        .build(|| {
                            if let Some((example, progress)) = self.inputs_outputs_example.as_ref()
                            {
                                self.ui_show_example(example, progress.as_ref(), ui);
                            }

                            if self.exec.is_none() && ui.button(im_str!("Generate"), [80.0, 20.0]) {
                                updated_example =
                                    self.editor.machine().level.as_ref().map(|level| {
                                        level.spec.gen_inputs_outputs(&mut rand::thread_rng())
                                    });
                            }
                        });
                });

            if let Some(example) = updated_example {
                self.inputs_outputs_example = Some((example, None));
            }
        }
    }

    fn ui_show_example(
        &self,
        example: &level::InputsOutputs,
        progress: Option<&InputsOutputsProgress>,
        ui: &imgui::Ui,
    ) {
        for (index, row) in example.inputs.iter().enumerate() {
            let input_progress = progress
                .and_then(|progress| progress.inputs.get(index).copied())
                .unwrap_or(0);
            let input_failed = false; // Input can't fail

            self.ui_show_blip_row(
                &format!("In {}", index),
                row.iter().copied(),
                input_progress,
                input_failed,
                ui,
            );
        }

        //ui.separator();

        for (index, row) in example.outputs.iter().enumerate() {
            let output_progress = progress
                .and_then(|progress| progress.outputs.get(index).copied())
                .unwrap_or(0);
            let output_failed = progress
                .and_then(|progress| progress.outputs_failed.get(index).copied())
                .unwrap_or(false);

            self.ui_show_blip_row(
                &format!("Out {}", index),
                row.iter().map(|kind| Some(level::Input::Blip(*kind))),
                output_progress,
                output_failed,
                ui,
            );
        }
    }

    fn ui_show_blip_row(
        &self,
        label: &str,
        row: impl Iterator<Item = Option<level::Input>>,
        progress: usize,
        failed: bool,
        ui: &imgui::Ui,
    ) {
        let border_margin = 2.0;
        let progress_color = [1.0, 1.0, 1.0];
        let failed_color = [1.0, 0.0, 0.0];
        let blip_size = 16.0;

        let draw_list = ui.get_window_draw_list();

        ui.text(&ImString::new(label));

        for (column, input) in row.enumerate() {
            ui.same_line(if column == 0 { 80.0 } else { 0.0 });

            match input {
                Some(level::Input::Blip(kind)) => {
                    let color: [f32; 3] = machine::render::blip_color(kind).into();
                    let cursor_pos = ui.cursor_screen_pos();

                    let border_a = [cursor_pos[0] - border_margin, cursor_pos[1] - border_margin];
                    let border_b = [
                        cursor_pos[0] + blip_size + border_margin,
                        cursor_pos[1] + blip_size + border_margin,
                    ];

                    if !failed && progress == column + 1 {
                        draw_list
                            .add_rect(border_a, border_b, progress_color)
                            .build();
                    } else if failed && progress == column {
                        draw_list.add_rect_filled_multicolor(
                            border_a,
                            border_b,
                            failed_color,
                            failed_color,
                            failed_color,
                            failed_color,
                        );
                    }

                    draw_list.add_rect_filled_multicolor(
                        cursor_pos,
                        [cursor_pos[0] + blip_size, cursor_pos[1] + blip_size],
                        color,
                        color,
                        color,
                        color,
                    );
                }
                None => (),
            }

            ui.dummy([blip_size, blip_size]);
        }
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

/// `InputsOutputsProgress` stores the progress through the current
/// `InputsOutputs` example while executing. The state is entirely derived from
/// the machine's execution state. We store it, so that the user can see where
/// execution failed even while editing afterwards.
struct InputsOutputsProgress {
    /// How many inputs have been fed by index?
    ///
    /// This vector has the same length as the level's `InputOutputs::inputs`.
    inputs: Vec<usize>,

    /// How many outputs have been correctly fed by index?
    ///
    /// This vector has the same length as the level's `InputOutputs::outputs`.
    outputs: Vec<usize>,

    /// Which outputs have failed (in their last time step)?
    ///
    /// This vector has the same length as the level's `InputOutputs::outputs`.
    outputs_failed: Vec<bool>,
}

impl InputsOutputsProgress {
    pub fn new_from_exec(example: &level::InputsOutputs, exec: &Exec) -> Self {
        let machine = exec.machine();
        let inputs = example
            .inputs
            .iter()
            .enumerate()
            .map(|(i, spec)| {
                let progress = machine
                    .blocks
                    .data
                    .values()
                    .find_map(|(_block_pos, block)| {
                        // Block::Input index is assumed to be unique within
                        // the machine
                        match &block.block {
                            Block::Input { index, inputs, .. } if *index == i => {
                                // Note that `inputs` here stores the remaining
                                // inputs that will be fed into the machine.
                                Some(if spec.len() >= inputs.len() {
                                    spec.len() - inputs.len()
                                } else {
                                    // This case can only happen if `example`
                                    // comes from the wrong source, ignore
                                    0
                                })
                            }
                            _ => None,
                        }
                    });

                // Just show no progress if we ever have missing input blocks
                progress.unwrap_or(0)
            })
            .collect();

        let outputs_and_failed = example
            .outputs
            .iter()
            .enumerate()
            .map(|(i, spec)| {
                let progress = machine
                    .blocks
                    .data
                    .values()
                    .find_map(|(_block_pos, block)| {
                        // Block::Output index is assumed to be unique within
                        // the machine
                        match &block.block {
                            Block::Output {
                                index,
                                outputs,
                                activated,
                                failed,
                                ..
                            } if *index == i => {
                                // Note that `outputs` here stores the remaining
                                // outputs that need to come out of the machine.
                                let mut remaining = outputs.len();

                                // If `activated` matches the next expected
                                // output, there has been one more progress.
                                if remaining > 0
                                    && activated.is_some()
                                    && *activated == outputs.last().copied()
                                {
                                    remaining -= 1;
                                }

                                Some(if spec.len() >= remaining {
                                    (spec.len() - remaining, *failed)
                                } else {
                                    // This case can only happen if `example`
                                    // comes from the wrong source, ignore
                                    (0, false)
                                })
                            }
                            _ => None,
                        }
                    });

                // Just show no progress if we ever have missing input blocks
                progress.unwrap_or((0, false))
            })
            .collect::<Vec<_>>();

        Self {
            inputs,
            outputs: outputs_and_failed.iter().map(|(a, _)| *a).collect(),
            outputs_failed: outputs_and_failed.iter().map(|(_, b)| *b).collect(),
        }
    }
}

#[derive(Debug)]
pub enum CreationError {
    RenderPipeline(render::pipeline::CreationError),
    RenderResources(resources::CreationError),
}
