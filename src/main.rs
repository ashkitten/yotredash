#[macro_use]
extern crate glium;
#[macro_use]
extern crate clap;
extern crate time;

use glium::{glutin, Surface};
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}

struct Shape {
    vertex_buffer: glium::VertexBuffer<Vertex>,
    index_buffer: glium::index::NoIndices,
    shader_program: glium::Program,
}

struct Args {
    vertex: String,
    fragment: String,
    channels: Vec<String>,
    root: bool,
    override_redirect: bool,
    desktop: bool,
    lower_window: bool,
    width: u32,
    height: u32,
    maximize: bool,
    vsync: bool,
    fps: bool,
    font: String,
    verbose: u8,
}

fn parse_args() -> Args {
    // The YAML file is found relative to the current file, similar to how modules are found
    let yaml = load_yaml!("cli.yml");
    let matches = clap::App::from_yaml(yaml).get_matches();

    return Args {
        vertex: matches.value_of("vertex").unwrap_or("").to_owned(),
        fragment: matches.value_of("fragment").unwrap().to_owned(),
        channels: matches.values_of("channel").unwrap_or(clap::Values::default()).map(|channel: &str| channel.to_owned()).collect(),
        root: matches.is_present("root"),
        override_redirect: matches.is_present("override-redirect"),
        desktop: matches.is_present("desktop"),
        lower_window: matches.is_present("lower-window"),
        width: matches.value_of("width").unwrap_or("640").parse::<u32>().unwrap(),
        height: matches.value_of("height").unwrap_or("400").parse::<u32>().unwrap(),
        maximize: matches.is_present("maximize"),
        vsync: matches.is_present("vsync"),
        fps: matches.is_present("fps"),
        font: matches.value_of("font").unwrap_or("").to_owned(),
        verbose: matches.value_of("verbose").unwrap_or("0").parse::<u8>().unwrap(),
    }
}

fn init_display(args: &Args) -> (glutin::EventsLoop, glium::Display) {
    let events_loop = glutin::EventsLoop::new();
    let window_builder = glutin::WindowBuilder::new()
        .with_dimensions(args.width, args.height);

    let context = glutin::ContextBuilder::new();
    let display = glium::Display::new(window_builder, context, &events_loop).unwrap();

    return (events_loop, display);
}

fn init_gl(display: &glium::Display, args: &Args) -> Shape {
    implement_vertex!(Vertex, position);

    let vertices = [
        Vertex { position: [-1.0, -1.0] },
        Vertex { position: [ 1.0, -1.0] },
        Vertex { position: [ 1.0,  1.0] },
        Vertex { position: [-1.0,  1.0] },
    ];
    let triangles = vec![
        vertices[0], vertices[1], vertices[2],
        vertices[0], vertices[2], vertices[3]
    ];

    let vertex_buffer = glium::VertexBuffer::new(display, &triangles).unwrap();
    let index_buffer = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);


    let default_vertex_source = "
        #version 130

        in vec2 position;

        void main() {
            gl_Position = vec4(position, 0.0, 1.0);
        }
    ";

    let text_vertex_source = "
        #version 130

        in vec4 vertex; // <vec2 pos, vec2 tex>
        out vec2 texCoords;

        uniform mat4 projection;

        void main() {
            gl_Position = projection * vec4(vertex.xy, 0.0, 1.0);
            texCoords = vertex.zw;
        }
    ";

    let text_fragment_source = "
        #version 130

        in vec2 texCoords;
        out vec4 fragColor;

        uniform sampler2D glyphTexture;
        uniform vec3 color;

        void main() {
            vec4 sampled = vec4(1.0, 1.0, 1.0, texture(glyphTexture, texCoords).r);
            fragColor = vec4(color, 1.0) * sampled;
        }
    ";

    let file = File::open(&args.fragment).expect("File not found");
    let mut buf_reader = BufReader::new(file);
    let mut fragment_source = String::new();
    buf_reader.read_to_string(&mut fragment_source).expect("Could not read the fragment shader file");

    let mut vertex_source = String::new();
    if !args.vertex.is_empty() {
        let file = File::open(&args.vertex).expect("File not found");
        let mut buf_reader = BufReader::new(file);
        buf_reader.read_to_string(&mut vertex_source).expect("Could not read the vertex shader file");
    } else {
        vertex_source = default_vertex_source.to_owned();
    }

    let shader_program = glium::Program::from_source(display, &vertex_source, &fragment_source, None).unwrap();

    let shape = Shape {
        vertex_buffer: vertex_buffer,
        index_buffer: index_buffer,
        shader_program: shader_program,
    };

    return shape;
}

fn render(display: &glium::Display, shape: &Shape, start_time: &time::Tm) {
    let mut target = display.draw();
    target.clear_color(0.0, 0.0, 0.0, 1.0);

    let window_size = display.gl_window().get_inner_size_pixels().unwrap();

    let uniforms = uniform! {
        iResolution: (window_size.0 as f32, window_size.1 as f32),
        iTime: (((time::now() - *start_time).num_microseconds().unwrap() as f64) / 1000000.0) as f32,
    };

    target.draw(&shape.vertex_buffer, &shape.index_buffer, &shape.shader_program, &uniforms, &Default::default()).unwrap();
    target.finish().unwrap();
}

fn main() {
    let args = parse_args();
    let (mut events_loop, display) = init_display(&args);
    let shape = init_gl(&display, &args);

    let start_time = time::now();

    let mut closed = false;
    while !closed {
        render(&display, &shape, &start_time);

        events_loop.poll_events(|event| {
            match event {
                glutin::Event::WindowEvent { event, .. } => match event {
                    glutin::WindowEvent::Closed => closed = true,
                    glutin::WindowEvent::KeyboardInput { input: glutin::KeyboardInput { virtual_keycode: Some(glutin::VirtualKeyCode::Escape), .. }, .. } => closed = true,
                    _ => ()
                },
                _ => (),
            }
        });
    }
}
