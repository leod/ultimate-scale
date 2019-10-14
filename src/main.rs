#![feature(type_alias_impl_trait)]

mod config;
mod edit;
mod exec;
mod game;
mod machine;
mod render;
mod util;

use std::fs::File;
use std::io::BufReader;
use std::thread;
use std::time::{Duration, Instant};

use clap::{App, Arg};
use log::info;

use game::Game;
use machine::{grid, Machine, SavedMachine};

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

    let config: config::Config = Default::default();
    info!("Running with config: {:?}", config);

    let core =
        render::pipeline::simple::diffuse_core_transform(render::pipeline::simple::plain_core());
    let compilation = core.link().compile();

    println!("{}", compilation.vertex);
    println!("{}", compilation.fragment);

    info!("Opening window");
    let mut events_loop = glutin::EventsLoop::new();
    let display = {
        let window_builder = glutin::WindowBuilder::new()
            .with_dimensions(config.view.window_size)
            .with_title("Ultimate Scale!");
        let context_builder = glutin::ContextBuilder::new();
        glium::Display::new(window_builder, context_builder, &events_loop).unwrap()
    };

    let initial_machine = if let Some(file) = args.value_of("file") {
        info!("Loading machine from file `{}'", file);
        let file = File::open(file).unwrap();
        let reader = BufReader::new(file);
        let saved_machine: SavedMachine = serde_json::from_reader(reader).unwrap();
        saved_machine.into_machine()
    } else {
        let grid_size = grid::Vector3::new(30, 30, 4);
        Machine::new(grid_size)
    };

    let mut game = Game::create(&display, &config, initial_machine).unwrap();

    let mut previous_clock = Instant::now();
    let mut quit = false;

    while !quit {
        game.render(&display).unwrap();

        // Remember only the last (hopefully: newest) resize event. We do this
        // because resizing textures is somewhat costly, so it makes sense to
        // do it at most once per frame.
        let mut new_window_size = None;

        events_loop.poll_events(|event| match event {
            glutin::Event::WindowEvent { event, .. } => {
                game.on_event(&event);

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

            game.on_window_resize(&display, new_window_size);

            //font.on_window_resize(new_window_size);
        }

        let now_clock = Instant::now();
        let frame_duration = now_clock - previous_clock;
        previous_clock = now_clock;
        game.update(frame_duration);

        thread::sleep(Duration::from_millis(0));
    }
}
