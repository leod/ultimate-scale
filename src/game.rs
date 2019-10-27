use std::time::Duration;

use floating_duration::TimeAsFloat;
use log::info;
use nalgebra as na;

use glium::glutin;

use crate::config::{self, Config};
use crate::edit::Editor;
use crate::exec::{self, ExecView};
use crate::input_state::InputState;
use crate::machine::Machine;

use crate::render::camera::{Camera, EditCameraView, EditCameraViewInput};
use crate::render::pipeline::deferred::DeferredShading;
use crate::render::pipeline::shadow::{self, ShadowMapping};
use crate::render::pipeline::{Light, RenderLists};
use crate::render::Resources;
use crate::render::{self, resources};

#[derive(Debug)]
pub enum CreationError {
    ShadowMappingCreationError(shadow::CreationError),
    ResourcesCreationError(resources::CreationError),
}

pub struct Game {
    config: Config,

    resources: Resources,

    camera: Camera,
    edit_camera_view: EditCameraView,
    edit_camera_view_input: EditCameraViewInput,

    shadow_mapping: Option<ShadowMapping>,
    deferred_shading: Option<DeferredShading>,
    render_lists: RenderLists,

    editor: Editor,
    exec_view: Option<ExecView>,

    elapsed_time: Duration,
    fps: f32,
}

impl Game {
    pub fn create<F: glium::backend::Facade>(
        facade: &F,
        config: &Config,
        initial_machine: Machine,
    ) -> Result<Game, CreationError> {
        info!("Creating resources");
        let resources = Resources::create(facade)?;

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

        let shadow_mapping = config
            .render
            .shadow_mapping
            .as_ref()
            .map(|config| ShadowMapping::create(facade, config, false))
            .transpose()?;

        let deferred_shading = config
            .render
            .deferred_shading
            .as_ref()
            .map(|deferred_shading_config| {
                DeferredShading::create(
                    facade,
                    &deferred_shading_config,
                    config.view.window_size,
                    &config.render.shadow_mapping,
                )
            })
            .transpose()?;

        let render_lists = RenderLists::new();

        let editor = Editor::new(&config.editor, &config.exec, initial_machine);

        Ok(Game {
            config: config.clone(),
            resources,
            camera,
            edit_camera_view,
            edit_camera_view_input,
            shadow_mapping,
            deferred_shading,
            render_lists,
            editor,
            exec_view: None,
            elapsed_time: Default::default(),
            fps: 0.0,
        })
    }

    pub fn render<S: glium::Surface>(
        &mut self,
        display: &glium::backend::glutin::Display,
        target: &mut S,
    ) -> Result<(), glium::DrawError> {
        let render_context = render::pipeline::Context {
            camera: self.camera.clone(),
            elapsed_time_secs: self.elapsed_time.as_fractional_secs() as f32,
            tick_progress: self
                .exec_view
                .as_ref()
                .map_or(0.0, |exec_view| exec_view.cur_tick_progress()),
            main_light_pos: na::Point3::new(
                15.0 + 20.0 * (std::f32::consts::PI / 4.0).cos(),
                15.0 + 20.0 * (std::f32::consts::PI / 4.0).sin(),
                20.0,
            ),
            main_light_center: na::Point3::new(15.0, 15.0, 0.0),
        };

        self.render_lists.clear();

        target.clear_color_and_depth((0.0, 0.0, 0.0, 0.0), 1.0);

        if let Some(exec_view) = self.exec_view.as_mut() {
            exec_view.render(&mut self.render_lists);
        } else {
            self.editor.render(&mut self.render_lists)?;
        }

        if let Some(deferred_shading) = &mut self.deferred_shading {
            profile!("deferred");

            let intensity = 1.0;
            self.render_lists.lights.push(Light {
                position: render_context.main_light_pos,
                attenuation: na::Vector3::new(1.0, 0.01, 0.00001),
                color: na::Vector3::new(intensity, intensity, intensity),
                radius: 160.0,
            });

            deferred_shading.render_frame(
                display,
                &self.resources,
                &render_context,
                &self.render_lists,
                target,
            )?;
        } else if let Some(shadow_mapping) = &mut self.shadow_mapping {
            profile!("shadow");

            shadow_mapping.render_frame(
                display,
                &self.resources,
                &render_context,
                &self.render_lists,
                target,
            )?;
        } else {
            profile!("straight");

            render::pipeline::render_frame_straight(
                &self.resources,
                &render_context,
                &self.render_lists,
                target,
            )?;
        }

        Ok(())
    }

    pub fn update(&mut self, dt: Duration, input_state: &InputState) {
        self.elapsed_time += dt;
        let dt_secs = dt.as_fractional_secs() as f32;
        self.fps = 1.0 / dt_secs;

        if let Some(exec_view) = self.exec_view.as_mut() {
            exec_view.update(dt, &self.camera, &self.edit_camera_view);
        } else {
            self.exec_view = self.editor.update(
                dt_secs,
                input_state,
                &self.camera,
                &mut self.edit_camera_view,
            );
        }

        match self.exec_view.as_ref().map(|view| view.status()) {
            Some(exec::view::Status::Stopped) => {
                info!("Stopping exec, returning to editor");
                self.exec_view = None
            }
            _ => {}
        }

        self.edit_camera_view_input
            .update(dt_secs, input_state, &mut self.edit_camera_view);
        self.camera.view = self.edit_camera_view.view();
    }

    pub fn ui(&mut self, ui: &imgui::Ui) {
        imgui::Window::new(imgui::im_str!("Hello world"))
            .size([300.0, 100.0], imgui::Condition::FirstUseEver)
            .build(&ui, || {
                ui.text(imgui::im_str!("Hello world!"));
                ui.text(imgui::im_str!("こんにちは世界！"));
                ui.text(imgui::im_str!("This...is...imgui-rs!"));
                ui.separator();
                let mouse_pos = ui.io().mouse_pos;
                ui.text(format!(
                    "Mouse Position: ({:.1},{:.1})",
                    mouse_pos[0], mouse_pos[1]
                ));
            });

        if let Some(exec_view) = self.exec_view.as_mut() {
            exec_view.ui(ui);
        } else {
            self.editor.ui(ui);
        }
    }

    pub fn on_event(&mut self, event: &glutin::WindowEvent) {
        self.edit_camera_view_input.on_event(event);

        if let Some(exec_view) = self.exec_view.as_mut() {
            exec_view.on_event(event);
        } else {
            self.editor.on_event(event);
        }
    }

    pub fn on_window_resize<F: glium::backend::Facade>(
        &mut self,
        facade: &F,
        new_window_size: glutin::dpi::LogicalSize,
    ) {
        self.camera.projection = Self::perspective_matrix(&self.config.view, new_window_size);
        self.camera.viewport = na::Vector4::new(
            0.0,
            0.0,
            new_window_size.width as f32,
            new_window_size.height as f32,
        );

        if let Some(deferred_shading) = self.deferred_shading.as_mut() {
            deferred_shading
                .on_window_resize(facade, new_window_size)
                .unwrap();
        }
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

impl From<shadow::CreationError> for CreationError {
    fn from(err: shadow::CreationError) -> CreationError {
        CreationError::ShadowMappingCreationError(err)
    }
}

impl From<resources::CreationError> for CreationError {
    fn from(err: resources::CreationError) -> CreationError {
        CreationError::ResourcesCreationError(err)
    }
}
