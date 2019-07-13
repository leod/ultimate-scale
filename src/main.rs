mod grid;
mod machine;
mod vec_option;
mod render;

use std::{thread, time::Duration};

use log::info;
use nalgebra as na;
use glium::Surface;

#[derive(Debug, Clone)]
struct Config {
    window_size: glutin::dpi::LogicalSize,
    fov_degrees: f64,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            window_size: glutin::dpi::LogicalSize::new(1024.0, 768.0),
            fov_degrees: 90.0,
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

    let resources = render::Resources::create(&display).unwrap();

    let mut quit = false;
    let mut render_list = render::RenderList::new();

    let projection = na::Perspective3::new(
        config.window_size.width as f32 / config.window_size.height as f32,
        config.fov_degrees.to_radians() as f32,
        1.0,
        1000.0,
    );
    let mut camera = render::camera::Camera::from_projection(projection.to_homogeneous());

    while !quit {
        thread::sleep(Duration::from_millis(10));
        
        let mut target = display.draw();
        target.clear_color_and_depth((0.0, 0.0, 0.0, 0.0), 1.0);

        render_list.render(&resources, &camera, &mut target).unwrap();

        target.finish().unwrap();

        events_loop.poll_events(|event| {
            match event {
                glutin::Event::WindowEvent { event, .. } => match event {
                    glutin::WindowEvent::CloseRequested => {
                        info!("Quitting");

                        quit = true;
                    }
                    _ => (),
                }
                _ => (),
            }
        });
    }
}
