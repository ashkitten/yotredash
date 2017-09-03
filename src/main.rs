#[cfg(unix)]
extern crate signal;

#[macro_use]
extern crate glium;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

extern crate clap;
extern crate env_logger;
extern crate time;

mod buffer;
mod config;
mod platform;
mod uniforms;

#[cfg(unix)]
use signal::Signal;
#[cfg(unix)]
use signal::trap::Trap;

use glium::glutin;

use buffer::Buffer;
use config::Config;
use platform::display::DisplayExt;

#[derive(Copy, Clone)]
pub struct Vertex {
    position: [f32; 2],
}

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
    main_buffer.finish();
    target.finish().unwrap();
}

fn main() {
    // I don't even know why it wants us to use the result
    // Let's just tuck that away so we never have to see it again
    let _ = env_logger::init();

    // Register signal handler
    #[cfg(unix)]
    let trap = Trap::trap(&[Signal::SIGUSR1, Signal::SIGUSR2, Signal::SIGHUP]);

    let mut config = Config::parse();

    let mut events_loop = glutin::EventsLoop::new();
    let display = DisplayExt::init(&events_loop, &config);
    let (vertex_buffer, index_buffer) = init_gl(&display);

    let mut main_buffer = Buffer::new(&display, &config, "__default__");

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
                        config = Config::parse();
                        main_buffer = Buffer::new(&display, &config, "__default__");
                        start_time = time::now();
                    }
                    _ => (),
                }
            }
        }

        if config.fps {
            let delta = time::now() - last_frame;
            frames += 1.0;
            if delta > time::Duration::seconds(5) {
                println!("FPS: {}", frames / (delta.num_nanoseconds().unwrap() as f64 / 1_000_000_000.0));
                last_frame = time::now();
                frames = 0.0;
            }
        }

        events_loop.poll_events(|event| if let glutin::Event::WindowEvent { event, .. } = event {
            match event {
                glutin::WindowEvent::Resized(width, height) => {
                    main_buffer.resize(&display, width, height);
                }
                glutin::WindowEvent::Closed |
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
            }
        });
    }
}
