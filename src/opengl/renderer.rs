use glium::VertexBuffer;
use glium::backend::glutin::Display;
use glium::framebuffer::SimpleFrameBuffer;
use glium::glutin::{ContextBuilder, Window, WindowBuilder};
use glium::index::{NoIndices, PrimitiveType};
use glium::texture::{RawImage2d, Texture2d};
use image;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;
use std::rc::Rc;
use time;
use winit::EventsLoop;

use super::buffer::Buffer;
use config::Config;
use platform;

#[derive(Copy, Clone)]
pub struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

pub struct Renderer {
    display: Display,
    vertex_buffer: VertexBuffer<Vertex>,
    index_buffer: NoIndices,
    buffers: BTreeMap<String, Rc<RefCell<Buffer>>>,
    textures: BTreeMap<String, Rc<Texture2d>>,
}

impl Renderer {
    pub fn new(config: &Config, events_loop: &EventsLoop) -> Self {
        let window_builder = WindowBuilder::new().with_title("yotredash");
        let context_builder = ContextBuilder::new().with_vsync(config.vsync);
        let display = Display::new(window_builder, context_builder, &events_loop).unwrap();
        platform::window::init(display.gl_window().window(), config);

        #[cfg_attr(rustfmt, rustfmt_skip)]
        let vertices = [
            Vertex { position: [-1.0, -1.0] },
            Vertex { position: [ 1.0, -1.0] },
            Vertex { position: [ 1.0,  1.0] },
            Vertex { position: [-1.0, -1.0] },
            Vertex { position: [ 1.0,  1.0] },
            Vertex { position: [-1.0,  1.0] },
        ];

        let vertex_buffer = VertexBuffer::new(&display, &vertices).unwrap();
        let index_buffer = NoIndices(PrimitiveType::TrianglesList);

        let mut textures = BTreeMap::new();

        for (name, tconfig) in &config.textures {
            textures.insert(name.to_string(), {
                let image = image::open(Path::new(&tconfig.path)).unwrap().to_rgba();
                let image_dimensions = image.dimensions();
                let image = RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
                Rc::new(Texture2d::new(&display, image).unwrap())
            });
        }

        let mut buffers = BTreeMap::new();

        for (name, bconfig) in &config.buffers {
            buffers.insert(
                name.to_string(),
                Rc::new(RefCell::new(Buffer::new(
                    &display,
                    &bconfig,
                    bconfig
                        .textures
                        .iter()
                        .map(|name| Rc::clone(&textures[name]))
                        .collect(),
                ))),
            );
        }

        for (name, bconfig) in &config.buffers {
            buffers[name].borrow_mut().link_depends(&mut bconfig
                .depends
                .iter()
                .map(|name| Rc::clone(&buffers[name]))
                .collect());
        }

        Renderer {
            display: display,
            vertex_buffer: vertex_buffer,
            index_buffer: index_buffer,
            buffers: buffers,
            textures: textures,
        }
    }

    pub fn render(&self) {
        // TODO: implement
        let time = 0.0;
        let pointer = [0.0, 0.0, 0.0, 0.0];

        let mut target = self.display.draw();
        self.buffers["__default__"].borrow().render_to(
            &mut target,
            &self.vertex_buffer,
            &self.index_buffer,
            time,
            pointer,
        );
        target.finish().unwrap();
    }

    pub fn reload(&self, config: &Config) {}

    pub fn resize(&self, width: u32, height: u32) {}
}
