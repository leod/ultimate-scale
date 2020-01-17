// Needed for pareen stuff
#![type_length_limit = "60000000"]

//#![feature(type_alias_impl_trait)]

#[macro_use]
mod util;
mod config;
mod edit;
mod edit_camera_view;
mod exec;
mod game;
mod input_state;
mod machine;
mod render;

use std::fs::File;
use std::io::BufReader;
use std::thread;
use std::time::{Duration, Instant};

use clap::{App, Arg};
use coarse_prof::profile;
use glium::glutin;
use log::info;

use game::Game;
use input_state::InputState;
use machine::level::{Level, Spec};
use machine::{grid, BlipKind, Machine, SavedMachine};

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
        .arg(
            Arg::with_name("level")
                .short("l")
                .long("level")
                .value_name("LEVEL")
                .help("Play a specific level")
                .takes_value(true),
        )
        .get_matches();

    let mut config: config::Config = Default::default();
    config.render_pipeline.hdr = Some(1.0);
    info!("Running with config: {:?}", config);

    info!("Opening glutin window");
    let mut events_loop = glutin::EventsLoop::new();
    let display = {
        let window_builder = glutin::WindowBuilder::new()
            .with_dimensions(config.view.window_size)
            .with_title("Ultimate Scale!");
            //.with_fullscreen(Some(events_loop.get_primary_monitor()));
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
        let font_size = (14.0 * hidpi_factor) as f32;

        // Include some special characters in the glyph ranges
        let glyph_ranges = imgui::FontGlyphRanges::from_slice(&[
            0x0020, 0x00FF, // Basic Latin + Latin Supplement
            0,
        ]);

        // Symbola has some additional symbols that DeJaVu lacks
        let glyph_ranges_symbola = imgui::FontGlyphRanges::from_slice(&[
            0x2190, 0x21FF, // Arrows
            0x2300, 0x23FF, // Miscellaneous technical
            0x25A0, 0x25FF, // Geometric shapes
            0,
        ]);

        imgui.fonts().add_font(&[
            imgui::FontSource::TtfData {
                data: include_bytes!("../resources/DejaVuSans.ttf"),
                size_pixels: font_size,
                config: Some(imgui::FontConfig {
                    glyph_ranges,
                    ..imgui::FontConfig::default()
                }),
            },
            imgui::FontSource::TtfData {
                data: include_bytes!("../resources/Symbola_hint.ttf"),
                size_pixels: font_size,
                config: Some(imgui::FontConfig {
                    glyph_ranges: glyph_ranges_symbola,
                    ..imgui::FontConfig::default()
                }),
            },
        ]);

        imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;
    }

    let mut imgui_renderer = imgui_glium_renderer::Renderer::init(&mut imgui, &display)
        .expect("Failed to initialize imgui_glium_renderer");

    // TODO: Better level choosing
    let level = if let Some(level) = args.value_of("level") {
        if level == "id_3" {
            Some(Level {
                size: grid::Vector3::new(27, 27, 4),
                spec: Spec::Id { dim: 3 },
            })
        } else if level == "clock" {
            Some(Level {
                size: grid::Vector3::new(9, 9, 1),
                spec: Spec::Clock {
                    pattern: vec![BlipKind::A, BlipKind::B],
                },
            })
        } else if level == "o_beats_g" {
            Some(Level {
                size: grid::Vector3::new(19, 19, 2),
                spec: Spec::BitwiseMax,
            })
        } else if level == "make_it_3" {
            Some(Level {
                size: grid::Vector3::new(19, 19, 2),
                spec: Spec::MakeItN { n: 3, max: 30 },
            })
        } else if level == "mul_by_3" {
            Some(Level {
                size: grid::Vector3::new(19, 19, 2),
                spec: Spec::MultiplyByN { n: 3, max: 15 },
            })
        } else {
            None
        }
    } else {
        None
    };

    let initial_machine = if let Some(file) = args.value_of("file") {
        info!("Loading machine from file `{}'", file);
        let file = File::open(file).unwrap();
        let reader = BufReader::new(file);
        let saved_machine: SavedMachine = serde_json::from_reader(reader).unwrap();
        saved_machine.into_machine()
    } else if let Some(level) = level {
        info!("Running level \"{}\"", level.spec.description());
        Machine::new_from_level(level)
    } else {
        info!("Starting in sandbox mode");
        let grid_size = grid::Vector3::new(30, 30, 4);
        Machine::new_sandbox(grid_size)
    };

    let mut input_state = InputState::new();

    let mut game = Game::create(&display, &config, initial_machine).unwrap();

    let mut previous_clock = Instant::now();
    let mut previous_clock_imgui = Instant::now();
    let mut quit = false;

    while !quit {
        profile!("frame");

        // Remember only the last (hopefully: newest) resize event. We do this
        // because resizing textures is somewhat costly, so it makes sense to
        // do it at most once per frame.
        let mut new_window_size = None;

        events_loop.poll_events(|event| {
            imgui_platform.handle_event(imgui.io_mut(), &window, &event);

            match event {
                glutin::Event::Suspended(_) => {
                    input_state.clear();
                }
                glutin::Event::WindowEvent { event, .. } => {
                    // Do not forward events to the game if imgui currently
                    // wants to handle events (i.e. when the mouse is over a
                    // window).
                    //
                    // Note that we always forward release events to the game,
                    // so that e.g. we do not keep scrolling the view camera.
                    let forward_to_game = match event {
                        glutin::WindowEvent::KeyboardInput { input, .. } => {
                            !imgui.io().want_capture_keyboard
                                || input.state == glutin::ElementState::Released
                        }
                        glutin::WindowEvent::MouseInput { state, .. } => {
                            !imgui.io().want_capture_mouse
                                || state == glutin::ElementState::Released
                        }
                        _ => true,
                    };

                    if forward_to_game {
                        input_state.on_event(&event);
                        game.on_event(&input_state, &event);
                    }

                    match event {
                        glutin::WindowEvent::Focused(false) => {
                            input_state.clear();
                        }
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
                                        coarse_prof::write(&mut std::io::stdout()).unwrap();
                                        coarse_prof::reset();
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

            game.on_window_resize(&display, new_window_size).unwrap();
        }

        let now_clock = Instant::now();
        let frame_duration = now_clock - previous_clock;
        previous_clock = now_clock;

        {
            profile!("update");
            game.update(frame_duration, &input_state);
        }

        let ui_draw_data = {
            profile!("ui");

            let imgui_io = imgui.io_mut();
            imgui_platform
                .prepare_frame(imgui_io, &window)
                .expect("Failed to start imgui frame");
            previous_clock_imgui = imgui_io.update_delta_time(previous_clock_imgui);
            let ui = imgui.frame();
            game.ui(&ui);

            imgui_platform.prepare_render(&ui, &window);
            ui.render()
        };

        {
            profile!("draw");

            let mut target = {
                profile!("lock");
                display.draw()
            };

            {
                profile!("update_resources");
                game.update_resources(&display).unwrap();
            }

            game.draw(&display, &mut target).unwrap();

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
