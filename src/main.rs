mod grid;
mod machine;
mod vec_option;
mod render;

use log::info;
use std::{thread, time::Duration};

fn main() {
    simple_logger::init().unwrap();

    info!("Opening window");

    let mut events_loop = glutin::EventsLoop::new();
    let window_builder = glutin::WindowBuilder::new();
    let context_builder = glutin::ContextBuilder::new();
    let display = glium::Display::new(window_builder, context_builder, &events_loop).unwrap();

    let mut quit = false;
    while !quit {
        thread::sleep(Duration::from_millis(10));

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
