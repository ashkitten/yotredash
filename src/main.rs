#[macro_use]
extern crate glium;
#[macro_use]
extern crate clap;
extern crate time;
extern crate image;

// Glium
use glium::{glutin, Surface};
use glium::uniforms::{Uniforms,AsUniformValue,UniformValue};
// Std
use std::path::Path;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::borrow::Cow;

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
    fullscreen: bool,
    vsync: bool,
    fps: bool,
    font: String,
    verbose: u8,
}

struct UniformsStorageVec<'name,'uniform>(Vec<(Cow<'name, str>, Box<AsUniformValue + 'uniform>)>);

impl<'name,'uniform> UniformsStorageVec<'name,'uniform> {
    pub fn new() -> Self {
        UniformsStorageVec(Vec::new())
    }

    pub fn push<S: Into<Cow<'name, str>>, U: AsUniformValue + 'uniform>(&mut self, name: S, uniform: U) {
        self.0.push((name.into(), Box::new(uniform)))
    }
}

impl<'name,'uniform> Uniforms for UniformsStorageVec<'name,'uniform> {
    #[inline]
    fn visit_values<'a, F: FnMut(&str, UniformValue<'a>)>(&'a self, mut output: F) {
        for &(ref name, ref uniform) in &self.0 {
            output(name, uniform.as_uniform_value());
        }
    }
}

fn parse_args() -> Args {
    // The YAML file is found relative to the current file, similar to how modules are found
    // TODO: load different files based on detected features
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
        fullscreen: matches.is_present("fullscreen"),
        vsync: matches.is_present("vsync"),
        fps: matches.is_present("fps"),
        font: matches.value_of("font").unwrap_or("").to_owned(),
        verbose: matches.value_of("verbose").unwrap_or("0").parse::<u8>().unwrap(),
    }
}

fn override_redirect(display: &glium::Display) {
    // Use Unix-specific version of Window
    use glutin::os::unix::WindowExt;
    // For convenience
    use glutin::os::unix::x11::ffi::{Display, XID, CWOverrideRedirect, XSetWindowAttributes};

    // Get info about our connection, display, and window
    let x_connection = display.gl_window().get_xlib_xconnection().unwrap();
    let x_display = display.gl_window().get_xlib_display().unwrap() as *mut Display;
    let x_window = display.gl_window().get_xlib_window().unwrap() as XID;

    unsafe {
        // Change the override-redirect attribute
        (x_connection.xlib.XChangeWindowAttributes)(
            x_display,
            x_window,
            CWOverrideRedirect,
            &mut XSetWindowAttributes {
                background_pixmap: 0,
                background_pixel: 0,
                border_pixmap: 0,
                border_pixel: 0,
                bit_gravity: 0,
                win_gravity: 0,
                backing_store: 0,
                backing_planes: 0,
                backing_pixel: 0,
                save_under: 0,
                event_mask: 0,
                do_not_propagate_mask: 0,
                override_redirect: 1,
                colormap: 0,
                cursor: 0,
            }
        );
        // Remap the window so the override-redirect attribute can take effect
        (x_connection.xlib.XUnmapWindow)(x_display, x_window); // Unmap window
        (x_connection.xlib.XSync)(x_display, 0); // Sync (dunno why this is needed tbh, but it doesn't work without)
        (x_connection.xlib.XMapWindow)(x_display, x_window); // Remap window
    }
}

fn lower_window(display: &glium::Display) {
    // Use Unix-specific version of Window
    use glutin::os::unix::WindowExt;
    // For convenience
    use glutin::os::unix::x11::ffi::{Display, XID};

    // Get info about our connection, display, and window
    let x_connection = display.gl_window().get_xlib_xconnection().unwrap();
    let x_display = display.gl_window().get_xlib_display().unwrap() as *mut Display;
    let x_window = display.gl_window().get_xlib_window().unwrap() as XID;

    unsafe {
        (x_connection.xlib.XLowerWindow)(x_display, x_window);
    }
}

fn desktop_window(display: &glium::Display) {
    // Use Unix-specific version of Window
    use glutin::os::unix::WindowExt;
    // For convenience
    use glutin::os::unix::x11::ffi::{Display, XID, Atom, XA_ATOM, PropModeReplace};
    use std::ffi::CString;

    // Get info about our connection, display, and window
    let x_connection = display.gl_window().get_xlib_xconnection().unwrap();
    let x_display = display.gl_window().get_xlib_display().unwrap() as *mut Display;
    let x_window = display.gl_window().get_xlib_window().unwrap() as XID;

    unsafe {
        let window_type = (x_connection.xlib.XInternAtom)(x_display, CString::new("_NET_WM_WINDOW_TYPE").unwrap().as_ptr(), 0);
        let window_type_desktop = (x_connection.xlib.XInternAtom)(x_display, CString::new("_NET_WM_WINDOW_TYPE_DESKTOP").unwrap().as_ptr(), 0);
        (x_connection.xlib.XChangeProperty)(x_display, x_window, window_type, XA_ATOM, 32, PropModeReplace, &window_type_desktop as *const u64 as *const u8, 1);
    }
}

fn init_display(args: &Args) -> (glutin::EventsLoop, glium::Display) {
    let events_loop = glutin::EventsLoop::new();
    let window_builder = glutin::WindowBuilder::new()
        .with_dimensions(args.width, args.height);

    let context = glutin::ContextBuilder::new();
    let display = glium::Display::new(window_builder, context, &events_loop).unwrap();

    if args.override_redirect {
        override_redirect(&display);

        // After remapping the window we need to set the size again
        display.gl_window().set_inner_size(args.width, args.height);
    }

    if args.lower_window {
        lower_window(&display);
    }

    if args.desktop {
        desktop_window(&display);
    }

    return (events_loop, display);
}

fn init_gl(display: &glium::Display, args: &Args) -> (Shape, Vec<glium::texture::Texture2d>) {
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
        vertex_source = include_str!("default.vert").to_owned();
    }

    let shader_program = glium::Program::from_source(display, &vertex_source, &fragment_source, None).unwrap();

    let textures = args.channels.iter().map(|path: &String| {
        let image = image::open(&Path::new(&path)).unwrap();
        let image = image.as_rgba8().unwrap().clone();
        let image_dimensions = image.dimensions();
        let image = glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
        return glium::texture::Texture2d::new(display, image).unwrap();
    }).collect();

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
    uniforms.push("time", (((time::now() - *start_time).num_microseconds().unwrap() as f64) / 1000000.0 % 4096.0) as f32);
    for (i, texture) in textures.iter().enumerate() {
        uniforms.push(format!("texture{}", i), texture);
    }

    target.draw(&shape.vertex_buffer, &shape.index_buffer, &shape.shader_program, &uniforms, &Default::default()).unwrap();
    target.finish().unwrap();
}

fn main() {
    let args = parse_args();
    let (mut events_loop, display) = init_display(&args);
    let (shape, textures) = init_gl(&display, &args);

    let start_time = time::now();

    let mut closed = false;
    while !closed {
        render(&display, &shape, &textures, &start_time);

        events_loop.poll_events(|event| {
            match event {
                glutin::Event::WindowEvent { event, .. } => match event {
                    glutin::WindowEvent::Closed => closed = true,
                    glutin::WindowEvent::KeyboardInput {
                        input: glutin::KeyboardInput {
                            virtual_keycode: Some(glutin::VirtualKeyCode::Escape), ..
                        }, ..
                    } => closed = true,
                    _ => ()
                },
                _ => (),
            }
        });
    }
}
