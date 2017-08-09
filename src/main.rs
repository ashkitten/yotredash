#[macro_use]
extern crate clap;
#[macro_use]
extern crate glium;
extern crate time;
extern crate image;

mod platform;
mod args;

// Glium

use glium::{glutin, Surface};
use glium::uniforms::{AsUniformValue, UniformValue, Uniforms};

// Clap

use clap::ArgMatches;

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

struct Shape {
    vertex_buffer: glium::VertexBuffer<Vertex>,
    index_buffer: glium::index::NoIndices,
    shader_program: glium::Program,
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

fn init_gl(display: &glium::Display, args: &ArgMatches) -> (Shape, Vec<glium::texture::Texture2d>) {
    implement_vertex!(Vertex, position);

    let vertices = [
        Vertex {
            position: [-1.0, -1.0],
        },
        Vertex {
            position: [1.0, -1.0],
        },
        Vertex {
            position: [1.0, 1.0],
        },
        Vertex {
            position: [-1.0, 1.0],
        },
    ];
    let triangles = vec![
        vertices[0],
        vertices[1],
        vertices[2],
        vertices[0],
        vertices[2],
        vertices[3],
    ];

    let vertex_buffer = glium::VertexBuffer::new(display, &triangles).unwrap();
    let index_buffer = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

    let file = match File::open(&args.value_of("fragment").unwrap()) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("Could not open fragment shader file: {}", error);
            std::process::exit(1);
        }
    };
    let mut buf_reader = BufReader::new(file);
    let mut fragment_source = String::new();
    match buf_reader.read_to_string(&mut fragment_source) {
        Ok(file) => println!("Using fragment shader: {}", args.value_of("fragment").unwrap()),
        Err(error) => {
            eprintln!("Could not read fragment shader file: {}", error);
            std::process::exit(1);
        }
    };

    let file = match File::open(&args.value_of("vertex").unwrap()) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("Could not open vertex shader file: {}", error);
            std::process::exit(1);
        }
    };
    let mut buf_reader = BufReader::new(file);
    let mut vertex_source = String::new();
    match buf_reader.read_to_string(&mut vertex_source) {
        Ok(file) => println!("Using vertex shader: {}", args.value_of("vertex").unwrap()),
        Err(error) => {
            eprintln!("Could not read vertex shader file: {}", error);
            std::process::exit(1);
        }
    };

    let shader_program = glium::Program::from_source(display, &vertex_source, &fragment_source, None);
    let shader_program = match shader_program {
        Ok(program) => program,
        Err(error) => {
            eprintln!("{}", error);
            std::process::exit(1);
        }
    };

    let textures = args.values_of("channels")
        .unwrap()
        .map(|path: &str| {
            let image = image::open(&Path::new(&path)).unwrap();
            let image = image.as_rgba8().unwrap().clone();
            let image_dimensions = image.dimensions();
            let image = glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
            return glium::texture::Texture2d::new(display, image).unwrap();
        })
        .collect();

    let shape = Shape {
        vertex_buffer: vertex_buffer,
        index_buffer: index_buffer,
        shader_program: shader_program,
    };

    return (shape, textures);
}

fn render(display: &glium::Display, shape: &Shape, textures: &Vec<glium::texture::Texture2d>, start_time: &time::Tm) {
    let mut target = display.draw();
    target.clear_color(0.0, 0.0, 0.0, 1.0);

    let window_size = display.gl_window().get_inner_size_pixels().unwrap();

    let mut uniforms = UniformsStorageVec::new();
    uniforms.push("resolution", (window_size.0 as f32, window_size.1 as f32));
    uniforms.push(
        "time",
        (((time::now() - *start_time).num_microseconds().unwrap() as f64) / 1000000.0 % 4096.0) as f32,
    );
    for (i, texture) in textures.iter().enumerate() {
        uniforms.push(format!("texture{}", i), texture);
    }

    target
        .draw(
            &shape.vertex_buffer,
            &shape.index_buffer,
            &shape.shader_program,
            &uniforms,
            &Default::default(),
        )
        .unwrap();
    target.finish().unwrap();
}

fn main() {
    let args = args::parse_args();
    let mut events_loop = glutin::EventsLoop::new();
    let display = DisplayExt::init(&events_loop, &args);
    let (shape, textures) = init_gl(&display, &args);

    let start_time = time::now();

    let mut closed = false;
    while !closed {
        render(&display, &shape, &textures, &start_time);

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
