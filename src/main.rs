mod config;
mod edit;
mod exec;
mod game_state;
mod machine;
mod render;
mod util;

use std::thread;
use std::time::{Duration, Instant};

use floating_duration::TimeAsFloat;
use glium::Surface;
use log::info;
use nalgebra as na;

use edit::Editor;
use exec::Exec;
use game_state::GameState;

fn main() {
    simple_logger::init().unwrap();

    let config: config::Config = Default::default();
    info!("Running with config: {:?}", config);

    info!("Opening window");
    let mut events_loop = glutin::EventsLoop::new();
    let window_builder = glutin::WindowBuilder::new().with_dimensions(config.view.window_size);
    let context_builder = glutin::ContextBuilder::new();
    let display = glium::Display::new(window_builder, context_builder, &events_loop).unwrap();

    info!("Creating resources");
    let resources = render::Resources::create(&display).unwrap();

    let mut quit = false;

    let viewport = na::Vector2::new(
        config.view.window_size.width as f32,
        config.view.window_size.height as f32,
    );
    let projection = na::Perspective3::new(
        viewport.x / viewport.y,
        config.view.fov_degrees.to_radians() as f32,
        0.1,
        10000.0,
    );
    let mut camera = render::camera::Camera::new(viewport, projection.to_homogeneous());
    let mut edit_camera_view = render::camera::EditCameraView::new();
    let mut camera_input = render::camera::Input::new(config.camera);

    let mut render_lists = render::RenderLists::new();
    let mut shadow_mapping = config
        .render
        .shadow_mapping
        .as_ref()
        .map(|config| render::shadow::ShadowMapping::create(&display, config).unwrap());

    let grid_size = machine::grid::Vector3::new(30, 30, 4);
    let mut game_state = GameState::Edit(Editor::new(config.editor, config.exec, grid_size));

    let mut previous_clock = Instant::now();
    let mut elapsed_time: Duration = Default::default();

    while !quit {
        let now_clock = Instant::now();
        let frame_duration = now_clock - previous_clock;
        previous_clock = now_clock;

        elapsed_time += frame_duration;
        let render_context = render::Context {
            camera: camera.clone(),
            elapsed_time_secs: elapsed_time.as_fractional_secs() as f32,
        };

        render_lists.clear();

        let mut target = display.draw();
        target.clear_color_and_depth((0.0, 0.0, 0.0, 0.0), 1.0);

        match &mut game_state {
            GameState::Edit(editor) => editor.render(&mut render_lists).unwrap(),
            GameState::Exec {
                exec_view,
                editor: _,
            } => exec_view.render(&mut render_lists),
        }

        if let Some(shadow_mapping) = &mut shadow_mapping {
            shadow_mapping
                .render_frame(
                    &display,
                    &resources,
                    &render_context,
                    &render_lists,
                    &mut target,
                )
                .unwrap();
        } else {
            render::render_frame_straight(&resources, &render_context, &render_lists, &mut target)
                .unwrap();
        }

        target.finish().unwrap();

        events_loop.poll_events(|event| match event {
            glutin::Event::WindowEvent { event, .. } => {
                camera_input.on_event(&event);

                match &mut game_state {
                    GameState::Edit(editor) => editor.on_event(&event),
                    GameState::Exec {
                        exec_view,
                        editor: _,
                    } => exec_view.on_event(&event),
                }

                match event {
                    glutin::WindowEvent::CloseRequested => {
                        info!("Quitting");

                        quit = true;
                    }
                    _ => (),
                }
            }
            _ => (),
        });

        let frame_duration_secs = frame_duration.as_fractional_secs() as f32;
        camera_input.update(frame_duration_secs, &mut edit_camera_view);
        camera.view = edit_camera_view.view();

        game_state = match game_state {
            GameState::Edit(editor) => {
                editor.update(frame_duration_secs, &camera, &mut edit_camera_view)
            }
            GameState::Exec { exec_view, editor } => exec_view.update(frame_duration_secs, editor),
        };

        thread::sleep(Duration::from_millis(10));
    }
}
