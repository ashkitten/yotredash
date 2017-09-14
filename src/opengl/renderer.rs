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

use super::buffer::Buffer;
use super::text_renderer::TextRenderer;
use Renderer;
use config::Config;
use errors::*;
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
    text_renderer: TextRenderer,
    fps_counter: FpsCounter,
}

fn init_buffers(config: &Config, display: &Display) -> Result<HashMap<String, Rc<RefCell<Buffer>>>> {
    let mut textures = HashMap::new();

    for (name, tconfig) in &config.textures {
        textures.insert(name.to_string(), {
            let image = ::image::open(Path::new(&tconfig.path))?.to_rgba();
            let image_dimensions = image.dimensions();
            let image = RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
            Rc::new(Texture2d::new(display, image)?)
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
            )?)),
        );
    }

    for (name, bconfig) in &config.buffers {
        buffers[name].borrow_mut().link_depends(&mut bconfig
            .depends
            .iter()
            .map(|name| Rc::clone(&buffers[name]))
            .collect());
    }

    Ok(buffers)
}

impl Renderer for OpenGLRenderer {
    fn new(config: Config, events_loop: &EventsLoop) -> Result<Self> {
        let window_builder = WindowBuilder::new().with_title("yotredash");
        let context_builder = ContextBuilder::new().with_vsync(config.vsync);
        let display = Display::new(window_builder, context_builder, &events_loop)?;
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

        let vertex_buffer = VertexBuffer::new(&display, &vertices)?;
        let index_buffer = NoIndices(PrimitiveType::TrianglesList);
        let buffers = init_buffers(&config, &display)?;
        // TODO: font should not be hardcoded
        let text_renderer =
            TextRenderer::new(&display, "/usr/share/fonts/adobe-source-code-pro/SourceCodePro-Regular.otf", 64)?;

        Ok(Self {
            config: config,
            display: display,
            vertex_buffer: vertex_buffer,
            index_buffer: index_buffer,
            buffers: buffers,
            start_time: ::time::now(),
            text_renderer: text_renderer,
            fps_counter: FpsCounter::new(1.0),
        })
    }

    fn render(&mut self, pointer: [f32; 4]) -> Result<()> {
        let mut target = self.display.draw();

        self.buffers["__default__"].borrow().render_to(
            &mut target,
            &self.vertex_buffer,
            &self.index_buffer,
            ((::time::now() - self.start_time).num_nanoseconds().unwrap() as f32) / 1000_000_000.0 % 4096.0,
            pointer,
        )?;

        if self.config.fps {
            self.fps_counter.next_frame();

            self.text_renderer.draw_text(
                &self.display,
                &mut target,
                &("FPS: ".to_string() + &format!("{:.1}", self.fps_counter.fps())),
                0.0,
                0.0,
                [1.0, 1.0, 1.0],
            )?;
        }

        target.finish()?;

        Ok(())
    }

    fn swap_buffers(&self) -> Result<()> {
        self.display.draw().finish()?;
        Ok(())
    }

    fn reload(&mut self, config: &Config) -> Result<()> {
        info!("Reloading config");
        self.buffers = init_buffers(config, &self.display)?;
        Ok(())
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        for buffer in self.buffers.values() {
            buffer.borrow_mut().resize(&self.display, width, height)?;
        }
        Ok(())
    }
}
