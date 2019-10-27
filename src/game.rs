use std::path::Path;
use std::time::Duration;

use floating_duration::TimeAsFloat;
use log::info;
use nalgebra as na;

use glium::glutin;
use glium::Surface;

use crate::config::{self, Config};
use crate::edit::Editor;
use crate::exec::{self, ExecView};
use crate::machine::Machine;

use crate::render::camera::{self, Camera, EditCameraView};
use crate::render::pipeline::deferred::DeferredShading;
use crate::render::pipeline::shadow::{self, ShadowMapping};
use crate::render::pipeline::{Light, RenderLists};
use crate::render::text::{self, Font};
use crate::render::Resources;
use crate::render::{self, resources};

#[derive(Debug)]
pub enum CreationError {
    ShadowMappingCreationError(shadow::CreationError),
    ResourcesCreationError(resources::CreationError),
    FontCreationError(text::CreationError),
}

pub struct Game {
    config: Config,

    resources: Resources,
    font: Font,

    camera: Camera,
    edit_camera_view: EditCameraView,
    camera_input: camera::Input,

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
        let font = Font::load(
            facade,
            Path::new("resources/Readiness-Regular.ttf"),
            config.view.window_size,
        )?;

        let viewport_size = na::Vector2::new(
            config.view.window_size.width as f32,
            config.view.window_size.height as f32,
        );
        let camera = Camera::new(
            viewport_size,
            Self::perspective_matrix(&config.view, config.view.window_size),
        );
        let edit_camera_view = EditCameraView::new();
        let camera_input = camera::Input::new(&config.camera);

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
            font,
            resources,
            camera,
            edit_camera_view,
            camera_input,
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

        self.font.draw(
            na::Vector2::new(0.01, 0.01),
            0.02,
            na::Vector4::new(1.0, 0.0, 0.0, 1.0),
            &format!("FPS: {:.0}", self.fps),
            target,
        );

        Ok(())
    }

    pub fn update(&mut self, dt: Duration) {
        self.elapsed_time += dt;
        let dt_secs = dt.as_fractional_secs() as f32;
        self.fps = 1.0 / dt_secs;

        if let Some(exec_view) = self.exec_view.as_mut() {
            exec_view.update(dt, &self.camera, &self.edit_camera_view);
        } else {
            self.exec_view = self
                .editor
                .update(dt_secs, &self.camera, &mut self.edit_camera_view);
        }

        match self.exec_view.as_ref().map(|view| view.status()) {
            Some(exec::view::Status::Stopped) => {
                info!("Stopping exec, returning to editor");
                self.exec_view = None
            }
            _ => {}
        }

        self.camera_input
            .update(dt_secs, &mut self.edit_camera_view);
        self.camera.view = self.edit_camera_view.view();
    }

    pub fn on_event(&mut self, event: &glutin::WindowEvent) {
        self.camera_input.on_event(event);

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

        self.font.on_window_resize(new_window_size);
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

impl From<text::CreationError> for CreationError {
    fn from(err: text::CreationError) -> CreationError {
        CreationError::FontCreationError(err)
    }
}
