#[macro_use]
extern crate glium;
#[macro_use]
extern crate log;
extern crate clap;
extern crate env_logger;
#[cfg(unix)]
extern crate signal;
extern crate time;
extern crate json;

mod args;
mod buffer;
mod platform;
mod uniforms;

// Glium

use glium::glutin;

// Signal

#[cfg(unix)]
use signal::Signal;
#[cfg(unix)]
use signal::trap::Trap;

// Std

use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

// Local

use buffer::Buffer;
use platform::display::DisplayExt;

// Structs

#[derive(Copy, Clone)]
pub struct Vertex {
    position: [f32; 2],
}

// Functions

fn init_gl(display: &glium::Display) -> (glium::VertexBuffer<Vertex>, glium::index::NoIndices) {
    implement_vertex!(Vertex, position);

    #[cfg_attr(rustfmt, rustfmt_skip)]
    let vertices = [
        Vertex { position: [-1.0, -1.0] },
        Vertex { position: [ 1.0, -1.0] },
        Vertex { position: [ 1.0,  1.0] },
        Vertex { position: [-1.0,  1.0] },
    ];

    #[cfg_attr(rustfmt, rustfmt_skip)]
    let triangles = vec![
        vertices[0], vertices[1], vertices[2],
        vertices[0], vertices[2], vertices[3],
    ];

    let vertex_buffer = glium::VertexBuffer::new(display, &triangles).unwrap();
    let index_buffer = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

    (vertex_buffer, index_buffer)
}

fn render(
    display: &glium::Display, main_buffer: &Buffer, vertex_buffer: &glium::VertexBuffer<Vertex>,
    index_buffer: &glium::index::NoIndices, start_time: &time::Tm, pointer: (f32, f32, f32, f32),
) {
    let time = (((time::now() - *start_time).num_nanoseconds().unwrap() as f64) / 1000_000_000.0 % 4096.0) as f32;

    let mut target = display.draw();
    main_buffer.render_to(&mut target, vertex_buffer, index_buffer, time, pointer);
    target.finish().unwrap();
}

fn parse_config(path: &str) -> json::JsonValue {
    match File::open(path) {
        Ok(file) => {
            let mut buf_reader = BufReader::new(file);
            let mut config = String::new();
            match buf_reader.read_to_string(&mut config) {
                Ok(_) => {
                    info!("Using config file: {}", path);
                    json::parse(&config).unwrap()
                }
                Err(error) => {
                    error!("Could not read config file: {}", error);
                    json::JsonValue::new_object()
                }
            }
        }
        Err(error) => {
            info!("Could not open config file: {}", error);
            json::JsonValue::new_object()
        }
    }
}

fn get_config() -> (json::JsonValue, PathBuf) {
    let args = args::parse_args();
    if let Some(path) = args.value_of("config") {
        (parse_config(path), Path::new(path).parent().unwrap().to_path_buf())
    } else {
        let mut config = json::JsonValue::new_object();
        args::apply_args(&args, &mut config);
        (config, std::env::current_dir().unwrap().to_path_buf())
    }
}

fn main() {
    // I don't even know why it wants us to use the result
    // Let's just tuck that away so we never have to see it again
    let _ = env_logger::init();

    // Register signal handler
    #[cfg(unix)]
    let trap = Trap::trap(&[Signal::SIGUSR1, Signal::SIGUSR2, Signal::SIGHUP]);

    let (mut config, base_dir) = get_config();

    let mut events_loop = glutin::EventsLoop::new();
    let display = DisplayExt::init(&events_loop, &config);
    let (vertex_buffer, index_buffer) = init_gl(&display); // Make it mutable so we can reassign it later

    let mut main_buffer = Buffer::new(&display, &config["output"], &base_dir);

    let mut start_time = time::now();
    let mut pointer = (0.0, 0.0, 0.0, 0.0);

    let mut closed = false;
    let mut paused = false;
    let mut last_frame = time::now();
    let mut frames = 0.0;
    while !closed {
        if !paused {
            render(&display, &main_buffer, &vertex_buffer, &index_buffer, &start_time, pointer);
        } else {
            // Tuck this value away too
            let _ = display.swap_buffers();
        }

        #[cfg(unix)]
        {
            // Catch signals between draw calls
            let signal = trap.wait(std::time::Instant::now());
            if signal.is_some() {
                match signal.unwrap() {
                    Signal::SIGUSR1 => paused = true,
                    Signal::SIGUSR2 => paused = false,
                    Signal::SIGHUP => {
                        info!("Restarting!");
                        config = get_config().0;
                        main_buffer = Buffer::new(&display, &config["output"], &base_dir);
                        start_time = time::now();
                    }
                    _ => (),
                }
            }
        }

        if config["fps"].as_bool().unwrap_or(false) {
            let delta = time::now() - last_frame;
            frames += 1.0;
            if delta > time::Duration::seconds(5) {
                println!("FPS: {}", frames / (delta.num_nanoseconds().unwrap() as f64 / 1_000_000_000.0));
                last_frame = time::now();
                frames = 0.0;
            }
        }

        events_loop.poll_events(|event| match event {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::Closed => closed = true,
                glutin::WindowEvent::Resized(width, height) => {
                    config["output"]["width"] = width.into();
                    config["output"]["height"] = height.into();
                    main_buffer.resize(&display, width, height);
                }
                glutin::WindowEvent::KeyboardInput {
                    input: glutin::KeyboardInput {
                        virtual_keycode: Some(glutin::VirtualKeyCode::Escape),
                        ..
                    },
                    ..
                } => closed = true,
                glutin::WindowEvent::MouseMoved { position, .. } => {
                    pointer = (position.0 as f32, position.1 as f32, pointer.2, pointer.3)
                }
                glutin::WindowEvent::MouseInput {
                    button: glutin::MouseButton::Left,
                    state,
                    ..
                } => match state {
                    glutin::ElementState::Pressed => {
                        pointer = (pointer.0 as f32, pointer.1 as f32, pointer.0 as f32, pointer.1 as f32)
                    }
                    glutin::ElementState::Released => pointer = (pointer.0 as f32, pointer.1 as f32, 0.0, 0.0),
                },
                _ => (),
            },
            _ => (),
        });
    }
}
