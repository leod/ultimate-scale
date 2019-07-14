mod util;
mod machine;
mod render;
mod edit;

use std::thread;
use std::time::{Duration, Instant};

use log::info;
use nalgebra as na;
use glium::Surface;
use floating_duration::TimeAsFloat;

#[derive(Debug, Clone)]
struct Config {
    window_size: glutin::dpi::LogicalSize,
    fov_degrees: f64,
    camera_input: render::camera::Config,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            window_size: glutin::dpi::LogicalSize::new(1024.0, 768.0),
            fov_degrees: 90.0,
            camera_input: Default::default(),
        }
    }
}

fn main() {
    simple_logger::init().unwrap();

    let config: Config = Default::default();
    info!("Running with config: {:?}", config);

    info!("Opening window");
    let mut events_loop = glutin::EventsLoop::new();
    let window_builder = glutin::WindowBuilder::new()
        .with_dimensions(config.window_size);
    let context_builder = glutin::ContextBuilder::new();
    let display = glium::Display::new(window_builder, context_builder, &events_loop).unwrap();

    info!("Creating resources");
    let resources = render::Resources::create(&display).unwrap();

    let mut quit = false;
    let mut render_list = render::RenderList::new();

    let viewport = na::Vector2::new(
        config.window_size.width as f32,
        config.window_size.height as f32
    );
    let projection = na::Perspective3::new(
        viewport.x / viewport.y,
        config.fov_degrees.to_radians() as f32,
        1.0,
        10000.0,
    );
    let mut camera = render::camera::Camera::new(viewport, projection.to_homogeneous());
    let mut camera_input = render::camera::Input::new(config.camera_input);

    let mut previous_clock = Instant::now();
    let mut elapsed_time: Duration = Default::default();

    let grid_size = machine::grid::Vec3::new(30, 30, 8);
    let mut editor = edit::Editor::new(grid_size);

    while !quit {
        let now_clock = Instant::now();
        let frame_duration = now_clock - previous_clock;
        previous_clock = now_clock;

        elapsed_time += frame_duration;
        let render_context = render::Context {
            camera: camera.clone(),
            elapsed_time_secs: elapsed_time.as_fractional_secs() as f32,
        };

        render_list.clear();
        render_list.add(render::Object::Cube, &render::InstanceParams {
            transform: na::Translation::from(na::Vector3::new(3.0, 0.0, 0.0)).to_homogeneous(),
            color: na::Vector4::new(1.0, 0.0, 0.0, 1.0),
        });

        render_list.add(render::Object::Triangle, &render::InstanceParams {
            transform: na::Matrix4::identity(), 
            color: na::Vector4::new(1.0, 0.0, 0.0, 1.0),
        });

        let mut target = display.draw();
        target.clear_color_and_depth((0.0, 0.0, 0.0, 0.0), 1.0);

        editor.render(&resources, &render_context, &mut target).unwrap();
        render_list.render(&resources, &render_context, &mut target).unwrap();

        target.finish().unwrap();

        events_loop.poll_events(|event| {
            match event {
                glutin::Event::WindowEvent { event, .. } => {
                    camera_input.on_event(&event);
                    editor.on_event(&event);

                    match event {
                        glutin::WindowEvent::CloseRequested => {
                            info!("Quitting");

                            quit = true;
                        }
                        _ => (),
                    }
                }
                _ => (),
            }
        });

        let frame_duration_secs = frame_duration.as_fractional_secs() as f32;
        camera_input.update(frame_duration_secs, &mut camera);

        editor.update(frame_duration_secs, &camera);

        thread::sleep(Duration::from_millis(10));
    }
}
