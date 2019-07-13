mod grid;
mod machine;
mod vec_option;
mod render;

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
    info!("Running with config {:?}", config);

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

    let projection = na::Perspective3::new(
        config.window_size.width as f32 / config.window_size.height as f32,
        config.fov_degrees.to_radians() as f32,
        1.0,
        1000.0,
    );
    let view = na::Isometry3::from_parts(
        na::Translation::from(na::Vector3::new(0.0, 0.0, 3.0)),
        na::UnitQuaternion::identity(),
    );
    let mut camera = render::camera::Camera::new(
        projection.to_homogeneous(),
        view,
    );
    let mut camera_input = render::camera::Input::new(config.camera_input);

    render_list.add(render::Object::Cube, &render::InstanceParams {
        transform: na::Matrix4::identity(), 
        color: na::Vector4::new(1.0, 0.0, 0.0, 1.0),
    });

    let mut previous_clock = Instant::now();

    while !quit {
        let now_clock = Instant::now();
        let frame_duration = now_clock - previous_clock;
        let frame_duration_secs = frame_duration.as_fractional_secs() as f32;
        previous_clock = now_clock;

        let mut target = display.draw();
        target.clear_color_and_depth((0.0, 0.0, 0.0, 0.0), 1.0);
        render_list.render(&resources, &camera, &mut target).unwrap();
        target.finish().unwrap();

        events_loop.poll_events(|event| {
            match event {
                glutin::Event::WindowEvent { event, .. } => {
                    camera_input.on_event(&event);

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

        camera_input.move_camera(frame_duration_secs, &mut camera);

        thread::sleep(Duration::from_millis(10));
    }
}
