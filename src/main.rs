#![feature(type_alias_impl_trait)]

#[macro_use]
mod util;
mod config;
mod edit;
mod exec;
mod game;
mod machine;
mod render;

use std::fs::File;
use std::io::BufReader;
use std::thread;
use std::time::{Duration, Instant};

use clap::{App, Arg};
use glium::glutin;
use log::info;

use game::Game;
use machine::{grid, Machine, SavedMachine};

fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();

    let args = App::new("Ultimate Scale")
        .version("0.0.1")
        .author("leod <subtle.frustration@proton.me>")
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

    info!("Opening glutin window");
    let mut events_loop = glutin::EventsLoop::new();
    let display = {
        let window_builder = glutin::WindowBuilder::new()
            .with_dimensions(config.view.window_size)
            .with_title("Ultimate Scale!");
        let context_builder = glutin::ContextBuilder::new();
        glium::Display::new(window_builder, context_builder, &events_loop).unwrap()
    };
    let gl_window = display.gl_window();
    let window = gl_window.window();

    info!("Initializing imgui");
    let mut imgui = imgui::Context::create();

    // Disable saving window positions etc. for now
    imgui.set_ini_filename(None);

    let mut imgui_platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
    imgui_platform.attach_window(
        imgui.io_mut(),
        &window,
        imgui_winit_support::HiDpiMode::Rounded,
    );

    {
        let hidpi_factor = imgui_platform.hidpi_factor();
        let font_size = (13.0 * hidpi_factor) as f32;

        imgui
            .fonts()
            .add_font(&[imgui::FontSource::DefaultFontData {
                config: Some(imgui::FontConfig {
                    size_pixels: font_size,
                    ..imgui::FontConfig::default()
                }),
            }]);

        imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;
    }

    let mut imgui_renderer = imgui_glium_renderer::Renderer::init(&mut imgui, &display)
        .expect("Failed to initialize imgui_glium_renderer");

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
    let mut previous_clock_imgui = Instant::now();
    let mut quit = false;

    while !quit {
        let _frame_guard = util::profile::start_frame();

        // Remember only the last (hopefully: newest) resize event. We do this
        // because resizing textures is somewhat costly, so it makes sense to
        // do it at most once per frame.
        let mut new_window_size = None;

        events_loop.poll_events(|event| {
            imgui_platform.handle_event(imgui.io_mut(), &window, &event);

            match event {
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
                        glutin::WindowEvent::KeyboardInput { input, .. } => {
                            if input.state == glutin::ElementState::Pressed {
                                match input.virtual_keycode {
                                    Some(glutin::VirtualKeyCode::P) => {
                                        util::profile::print(&mut std::io::stdout());
                                        util::profile::reset();
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => (),
                    }
                }
                _ => (),
            }
        });

        if let Some(new_window_size) = new_window_size {
            info!("Window resized to: {:?}", new_window_size);

            game.on_window_resize(&display, new_window_size);

            //font.on_window_resize(new_window_size);
        }

        let now_clock = Instant::now();
        let frame_duration = now_clock - previous_clock;
        previous_clock = now_clock;

        {
            profile!("update");
            game.update(frame_duration);
        }

        let ui_draw_data = {
            profile!("ui");

            let imgui_io = imgui.io_mut();
            imgui_platform
                .prepare_frame(imgui_io, &window)
                .expect("Failed to start imgui frame");
            previous_clock_imgui = imgui_io.update_delta_time(previous_clock_imgui);
            let mut ui = imgui.frame();

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

            imgui_platform.prepare_render(&ui, &window);
            ui.render()
        };

        {
            profile!("render");

            let mut target = display.draw();
            game.render(&display, &mut target).unwrap();

            {
                profile!("ui");
                imgui_renderer
                    .render(&mut target, &ui_draw_data)
                    .expect("Failed to render imgui frame");
            }

            {
                profile!("finish");
                target.finish().expect("Failed to swap buffers");
            }
        }

        thread::sleep(Duration::from_millis(0));
    }
}
