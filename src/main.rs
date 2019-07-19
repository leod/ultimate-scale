mod config;
mod edit;
mod exec;
mod game_state;
mod machine;
mod render;
mod util;

use std::fs::File;
use std::io::BufReader;
use std::thread;
use std::time::{Duration, Instant};

use clap::{App, Arg};
use floating_duration::TimeAsFloat;
use glium::Surface;
use log::info;
use nalgebra as na;

use edit::Editor;
use game_state::GameState;
use machine::{grid, Machine, SavedMachine};

fn perspective_matrix(
    config: &config::ViewConfig,
    window_size: &glutin::dpi::LogicalSize,
) -> na::Matrix4<f32> {
    let projection = na::Perspective3::new(
        window_size.width as f32 / window_size.height as f32,
        config.fov_degrees.to_radians() as f32,
        0.1,
        10000.0,
    );
    projection.to_homogeneous()
}

fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();

    let args = App::new("Ultimate Scale")
        .version("0.0.1")
        .author("Leonard Dahlmann <leo.dahlmann@gmail.com>")
        .arg(
            Arg::with_name("file")
                .short("f")
                .long("file")
                .value_name("FILE")
                .help("Load the given machine")
                .takes_value(true),
        )
        .get_matches();

    let mut config: config::Config = Default::default();
    info!("Running with config: {:?}", config);

    info!("Opening window");
    let mut events_loop = glutin::EventsLoop::new();
    let window_builder = glutin::WindowBuilder::new().with_dimensions(config.view.window_size);
    let context_builder = glutin::ContextBuilder::new();
    let display = glium::Display::new(window_builder, context_builder, &events_loop).unwrap();

    info!("Creating resources");
    let resources = render::Resources::create(&display).unwrap();

    let viewport_size = na::Vector2::new(
        config.view.window_size.width as f32,
        config.view.window_size.height as f32,
    );
    let mut camera = render::camera::Camera::new(
        viewport_size,
        perspective_matrix(&config.view, &config.view.window_size),
    );
    let mut edit_camera_view = render::camera::EditCameraView::new();
    let mut camera_input = render::camera::Input::new(&config.camera);

    let mut render_lists = render::RenderLists::new();
    let mut shadow_mapping = config
        .render
        .shadow_mapping
        .as_ref()
        .map(|config| render::shadow::ShadowMapping::create(&display, config, false).unwrap());

    let initial_machine = if let Some(file) = args.value_of("file") {
        let file = File::open(file).unwrap();
        let reader = BufReader::new(file);
        let saved_machine: SavedMachine = serde_json::from_reader(reader).unwrap();
        saved_machine.into_machine()
    } else {
        let grid_size = grid::Vector3::new(30, 30, 4);
        Machine::new(grid_size)
    };
    let editor = Editor::new(&config.editor, &config.exec, initial_machine);

    let mut game_state = GameState::Edit(editor);

    let mut previous_clock = Instant::now();
    let mut elapsed_time: Duration = Default::default();

    let mut quit = false;

    let mut deferred_shading = config
        .render
        .deferred_shading
        .as_ref()
        .map(|deferred_shading| {
            render::deferred::DeferredShading::create(
                &display,
                &deferred_shading,
                config.view.window_size,
                &config.render.shadow_mapping,
            )
            .unwrap()
        });

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
            GameState::Exec { exec_view, .. } => exec_view.render(&mut render_lists),
        }

        if let Some(deferred_shading) = &mut deferred_shading {
            // TODO: Sync light position with shadow mapping
            let light_x = 15.0 + 20.0 * (std::f32::consts::PI / 4.0).cos();
            let light_y = 15.0 + 20.0 * (std::f32::consts::PI / 4.0).sin();
            let light_z = 20.0;
            render_lists.lights.push(render::Light {
                position: na::Point3::new(light_x, light_y, light_z),
                attenuation: na::Vector3::new(1.0, 0.01, 0.00001),
                color: na::Vector3::new(1.0, 1.0, 1.0),
                radius: 160.0,
            });

            deferred_shading
                .render_frame(
                    &display,
                    &resources,
                    &render_context,
                    &render_lists,
                    &mut target,
                )
                .unwrap();
        } else if let Some(shadow_mapping) = &mut shadow_mapping {
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

        // Remember only the last (hopefully: newest) resize event. We do this
        // because resizing textures is somewhat costly, so it makes sense to
        // do it at most once per frame.
        let mut new_window_size = None;

        events_loop.poll_events(|event| match event {
            glutin::Event::WindowEvent { event, .. } => {
                camera_input.on_event(&event);

                match &mut game_state {
                    GameState::Edit(editor) => editor.on_event(&event),
                    GameState::Exec { exec_view, .. } => exec_view.on_event(&event),
                }

                match event {
                    glutin::WindowEvent::CloseRequested => {
                        info!("Quitting");

                        quit = true;
                    }
                    glutin::WindowEvent::Resized(viewport_size) => {
                        new_window_size = Some(viewport_size);
                    }
                    _ => (),
                }
            }
            _ => (),
        });

        if let Some(new_window_size) = new_window_size {
            info!("Window resized to: {:?}", new_window_size);

            camera.projection = perspective_matrix(&config.view, &new_window_size);
            camera.viewport = na::Vector4::new(
                0.0,
                0.0,
                new_window_size.width as f32,
                new_window_size.height as f32,
            );

            if let Some(deferred_shading) = &mut deferred_shading {
                deferred_shading.on_window_resize(&display, new_window_size).unwrap();
            }
        }

        let frame_duration_secs = frame_duration.as_fractional_secs() as f32;
        game_state = match game_state {
            GameState::Edit(editor) => {
                editor.update(frame_duration_secs, &camera, &mut edit_camera_view)
            }
            GameState::Exec { exec_view, editor } => exec_view.update(frame_duration, editor),
        };

        camera_input.update(frame_duration_secs, &mut edit_camera_view);
        camera.view = edit_camera_view.view();

        thread::sleep(Duration::from_millis(0));
    }
}
