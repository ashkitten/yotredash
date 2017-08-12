#[macro_use]
extern crate glium;
#[macro_use]
extern crate log;
extern crate clap;
extern crate env_logger;
extern crate image;
extern crate signal;
extern crate time;

mod args;
mod platform;

// Glium

use glium::{glutin, Surface};
use glium::uniforms::{AsUniformValue, UniformValue, Uniforms};

// Clap

use clap::ArgMatches;

// Signal

use signal::Signal;
use signal::trap::Trap;

// Std

use std::borrow::Cow;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::path::Path;

// Local

use platform::display::DisplayExt;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}

struct Quad {
    vertex_buffer: glium::VertexBuffer<Vertex>,
    index_buffer: glium::index::NoIndices,
    shader_program: glium::Program,
    textures: Vec<glium::texture::Texture2d>,
}

struct UniformsStorageVec<'name, 'uniform>(Vec<(Cow<'name, str>, Box<AsUniformValue + 'uniform>)>);

impl<'name, 'uniform> UniformsStorageVec<'name, 'uniform> {
    pub fn new() -> Self {
        UniformsStorageVec(Vec::new())
    }

    pub fn push<S, U>(&mut self, name: S, uniform: U)
    where
        S: Into<Cow<'name, str>>,
        U: AsUniformValue + 'uniform,
    {
        self.0.push((name.into(), Box::new(uniform)))
    }
}

impl<'name, 'uniform> Uniforms for UniformsStorageVec<'name, 'uniform> {
    #[inline]
    fn visit_values<'a, F: FnMut(&str, UniformValue<'a>)>(&'a self, mut output: F) {
        for &(ref name, ref uniform) in &self.0 {
            output(name, uniform.as_uniform_value());
        }
    }
}

fn init_gl(display: &glium::Display, args: &ArgMatches) -> Quad {
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

    let file = match File::open(&args.value_of("fragment").unwrap()) {
        Ok(file) => file,
        Err(error) => {
            error!("Could not open fragment shader file: {}", error);
            std::process::exit(1);
        }
    };
    let mut buf_reader = BufReader::new(file);
    let mut fragment_source = String::new();
    match buf_reader.read_to_string(&mut fragment_source) {
        Ok(_) => info!("Using fragment shader: {}", args.value_of("fragment").unwrap()),
        Err(error) => {
            error!("Could not read fragment shader file: {}", error);
            std::process::exit(1);
        }
    };

    let file = match File::open(&args.value_of("vertex").unwrap()) {
        Ok(file) => file,
        Err(error) => {
            error!("Could not open vertex shader file: {}", error);
            std::process::exit(1);
        }
    };
    let mut buf_reader = BufReader::new(file);
    let mut vertex_source = String::new();
    match buf_reader.read_to_string(&mut vertex_source) {
        Ok(_) => info!("Using vertex shader: {}", args.value_of("vertex").unwrap()),
        Err(error) => {
            error!("Could not read vertex shader file: {}", error);
            std::process::exit(1);
        }
    };

    let shader_program = glium::Program::from_source(display, &vertex_source, &fragment_source, None);
    let shader_program = match shader_program {
        Ok(program) => program,
        Err(error) => {
            error!("{}", error);
            std::process::exit(1);
        }
    };

    let textures = args.values_of("texture")
        .unwrap_or_default()
        .map(|path: &str| {
            let image = image::open(&Path::new(&path)).unwrap();
            let image = image.as_rgba8().unwrap().clone();
            let image_dimensions = image.dimensions();
            let image = glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
            glium::texture::Texture2d::new(display, image).unwrap()
        })
        .collect();

    return Quad {
        vertex_buffer: vertex_buffer,
        index_buffer: index_buffer,
        shader_program: shader_program,
        textures: textures,
    };
}

fn render(display: &glium::Display, quad: &Quad, start_time: &time::Tm) {
    let mut target = display.draw();
    target.clear_color(0.0, 0.0, 0.0, 1.0);

    let window_size = display.gl_window().get_inner_size_pixels().unwrap();

    let mut uniforms = UniformsStorageVec::new();
    uniforms.push("resolution", (window_size.0 as f32, window_size.1 as f32));
    uniforms
        .push("time", (((time::now() - *start_time).num_microseconds().unwrap() as f64) / 1000_000.0 % 4096.0) as f32);
    for (i, texture) in quad.textures.iter().enumerate() {
        uniforms.push(format!("texture{}", i), texture);
    }

    target
        .draw(&quad.vertex_buffer, &quad.index_buffer, &quad.shader_program, &uniforms, &Default::default())
        .unwrap();
    target.finish().unwrap();
}

fn main() {
    // I don't even know why it wants us to use the result
    // Let's just tuck that away so we never have to see it again
    let _ = env_logger::init();

    // Register signal handler
    let trap = Trap::trap(&[Signal::SIGUSR1, Signal::SIGUSR2, Signal::SIGHUP]);

    let args = args::parse_args();
    let mut events_loop = glutin::EventsLoop::new();
    let display = DisplayExt::init(&events_loop, &args);
    let mut quad = init_gl(&display, &args); // Make it mutable so we can reassign it later
    let mut start_time = time::now();

    let mut closed = false;
    let mut paused = false;
    while !closed {
        if !paused {
            render(&display, &quad, &start_time);
        } else {
            // Tuck this value away too
            let _ = display.swap_buffers();
        }

        // Catch signals between draw calls
        let signal = trap.wait(std::time::Instant::now());
        if signal.is_some() {
            match signal.unwrap() {
                Signal::SIGUSR1 => paused = true,
                Signal::SIGUSR2 => paused = false,
                Signal::SIGHUP => {
                    info!("Restarting!");
                    quad = init_gl(&display, &args);
                    start_time = time::now();
                }
                _ => (),
            }
        }

        events_loop.poll_events(|event| match event {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::Closed => closed = true,
                glutin::WindowEvent::KeyboardInput {
                    input: glutin::KeyboardInput {
                        virtual_keycode: Some(glutin::VirtualKeyCode::Escape),
                        ..
                    },
                    ..
                } => closed = true,
                _ => (),
            },
            _ => (),
        });
    }
}
