use glium::VertexBuffer;
use glium::backend::glutin::Display;
use glium::glutin::{ContextBuilder, WindowBuilder};
use glium::index::{NoIndices, PrimitiveType};
use glium::texture::{RawImage2d, Texture2d};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use time::Tm;
use winit::EventsLoop;

#[cfg(feature = "font-rendering")]
use super::text_renderer::TextRenderer;

use super::buffer::Buffer;
use config::Config;
use renderer::Renderer;
use util::FpsCounter;

#[derive(Copy, Clone)]
pub struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

pub struct OpenGLRenderer {
    config: Config,
    display: Display,
    vertex_buffer: VertexBuffer<Vertex>,
    index_buffer: NoIndices,
    buffers: HashMap<String, Rc<RefCell<Buffer>>>,
    start_time: Tm,
    fps_counter: FpsCounter,
    #[cfg(feature = "font-rendering")] text_renderer: TextRenderer,
}

fn init_buffers(config: &Config, display: &Display) -> HashMap<String, Rc<RefCell<Buffer>>> {
    let mut textures = HashMap::new();

    for (name, tconfig) in &config.textures {
        textures.insert(name.to_string(), {
            let image = ::image::open(Path::new(&tconfig.path)).unwrap().to_rgba();
            let image_dimensions = image.dimensions();
            let image = RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
            Rc::new(Texture2d::new(display, image).unwrap())
        });
    }

    let mut buffers = HashMap::new();

    for (name, bconfig) in &config.buffers {
        buffers.insert(
            name.to_string(),
            Rc::new(RefCell::new(Buffer::new(
                display,
                bconfig,
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

    buffers
}

impl Renderer for OpenGLRenderer {
    fn new(config: Config, events_loop: &EventsLoop) -> Self {
        let window_builder = WindowBuilder::new().with_title("yotredash");
        let context_builder = ContextBuilder::new().with_vsync(config.vsync);
        let display = Display::new(window_builder, context_builder, events_loop).unwrap();
        ::platform::window::init(display.gl_window().window(), &config);

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
        let buffers = init_buffers(&config, &display);

        // TODO: font should not be hardcoded
        #[cfg(feature = "font-rendering")]
        let text_renderer =
            TextRenderer::new(&display, "/usr/share/fonts/adobe-source-code-pro/SourceCodePro-Regular.otf", 64);

        Self {
            config: config,
            display: display,
            vertex_buffer: vertex_buffer,
            index_buffer: index_buffer,
            buffers: buffers,
            start_time: ::time::now(),
            fps_counter: FpsCounter::new(1.0),
            #[cfg(feature = "font-rendering")]
            text_renderer: text_renderer,
        }
    }

    fn render(&mut self, pointer: [f32; 4]) {
        let mut target = self.display.draw();

        self.buffers["__default__"].borrow().render_to(
            &mut target,
            &self.vertex_buffer,
            &self.index_buffer,
            ((::time::now() - self.start_time).num_nanoseconds().unwrap() as f32) / 1000_000_000.0 % 4096.0,
            pointer,
        );

        if self.config.fps {
            self.fps_counter.next_frame();

            #[cfg(feature = "font-rendering")]
            {
                self.text_renderer.draw_text(
                    &self.display,
                    &mut target,
                    &("FPS: ".to_string() + &format!("{:.1}", self.fps_counter.fps())),
                    0.0,
                    0.0,
                    [1.0, 1.0, 1.0],
                );
            }
        }

        target.finish().unwrap();
    }

    fn swap_buffers(&self) {
        self.display.draw().finish().unwrap();
    }

    fn reload(&mut self, config: &Config) {
        info!("Reloading config");
        self.buffers = init_buffers(config, &self.display);
    }

    fn resize(&mut self, width: u32, height: u32) {
        for buffer in self.buffers.values() {
            buffer.borrow_mut().resize(&self.display, width, height);
        }
    }
}
