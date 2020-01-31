use nalgebra as na;

use imgui::{im_str, ImString};

use rendology::fxaa;

use crate::edit::editor;
use crate::exec::{LevelProgress, LevelStatus};
use crate::game::Game;
use crate::machine::level;
use crate::render;

impl Game {
    pub fn ui(&mut self, ui: &imgui::Ui) {
        let editor_ui_input = self
            .last_output
            .as_ref()
            .and_then(|o| o.editor_ui_input.as_ref());

        if let Some(editor_ui_input) = editor_ui_input {
            editor::ui::run(
                editor_ui_input,
                ui,
                &mut self.next_input_stage.editor_ui_output,
            );
        }

        /*let play_state = self.exec.as_ref().map(|(play_state, _)| play_state);
        self.play.ui(window_size, play_state, ui);*/

        if self.show_config_ui {
            imgui::Window::new(im_str!("Config"))
                .horizontal_scrollbar(true)
                .position(
                    [self.target_size.0 as f32, 10.0],
                    imgui::Condition::FirstUseEver,
                )
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
                .position(
                    [self.target_size.0 as f32, 300.0],
                    imgui::Condition::FirstUseEver,
                )
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

        /*if let Some(level) = self.editor.machine().level.as_ref() {
            if let Some((_, exec)) = self.exec.as_ref() {
                // During execution, set the shown example to the generated
                // one. Also remember the progress, so that it can still be
                // shown after execution.
                self.level_example = exec.level_progress().cloned();
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
                            match exec.next_level_status() {
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
                            if let Some(example) = self.level_example.as_ref() {
                                self.ui_show_example(example, ui);
                            }

                            if self.exec.is_none() && ui.button(im_str!("Generate"), [80.0, 20.0]) {
                                updated_example =
                                    self.editor.machine().level.as_ref().map(|level| {
                                        let inputs_outputs =
                                            level.spec.gen_inputs_outputs(&mut rand::thread_rng());
                                        LevelProgress::new(None, inputs_outputs)
                                    });
                            }
                        });
                });

            if let Some(example) = updated_example {
                self.level_example = Some(example);
            }
        }*/
    }

    fn ui_show_example(&self, example: &LevelProgress, ui: &imgui::Ui) {
        for (index, (row, progress)) in example
            .inputs_outputs
            .inputs
            .iter()
            .zip(example.inputs.iter())
            .enumerate()
        {
            let input_failed = false; // Input can't fail

            self.ui_show_blip_row(
                &format!("In {}", index),
                row.iter().copied(),
                progress.num_fed,
                input_failed,
                ui,
            );
        }

        //ui.separator();

        for (index, (row, progress)) in example
            .inputs_outputs
            .outputs
            .iter()
            .zip(example.outputs.iter())
            .enumerate()
        {
            self.ui_show_blip_row(
                &format!("Out {}", index),
                row.iter().map(|kind| Some(level::Input::Blip(*kind))),
                progress.num_fed,
                progress.failed,
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
                    let color: [f32; 3] = render::machine::blip_color(kind).into();
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
}
