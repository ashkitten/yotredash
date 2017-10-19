use glium::VertexBuffer;
use glium::backend::Facade;
use glium::backend::glutin::Display;
use glium::backend::glutin::headless::Headless;
use glium::glutin::{ContextBuilder, HeadlessRendererBuilder, WindowBuilder};
use glium::index::{NoIndices, PrimitiveType};
use glium::texture::{RawImage2d};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use time::Tm;
use winit::EventsLoop;

use Renderer;
use config::Config;
use errors::*;
use super::buffer::Buffer;
use super::image::Image;
use super::text_renderer::TextRenderer;
use util::FpsCounter;

pub enum Backend {
    Display(Display),
    Headless(Headless),
}

impl AsRef<Facade> for Backend {
    fn as_ref(&self) -> &(Facade + 'static) {
        use self::Backend::*;
        match *self {
            Display(ref facade) => facade,
            Headless(ref facade) => facade,
        }
    }
}

#[derive(Copy, Clone)]
pub struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

pub struct OpenGLRenderer {
    config: Config,
    backend: Backend,
    vertex_buffer: VertexBuffer<Vertex>,
    index_buffer: NoIndices,
    buffers: HashMap<String, Rc<RefCell<Buffer>>>,
    start_time: Tm,
    text_renderer: TextRenderer,
    fps_counter: FpsCounter,
}

fn init_buffers(config: &Config, facade: &Facade) -> Result<HashMap<String, Rc<RefCell<Buffer>>>> {
    let mut textures = HashMap::new();

    for (name, tconfig) in &config.textures {
        textures.insert(name.to_string(), Rc::new(RefCell::new(Image::new(&config.path_to(&tconfig.path), facade)?)));
    }

    let mut buffers = HashMap::new();

    for (name, bconfig) in &config.buffers {
        buffers.insert(
            name.to_string(),
            Rc::new(RefCell::new(Buffer::new(
                facade,
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
        let width = config.buffers["__default__"].width;
        let height = config.buffers["__default__"].height;
        let backend = if config.show_window {
            let window_builder = WindowBuilder::new().with_title("yotredash")
                .with_maximized(config.maximize)
                .with_fullscreen(match config.fullscreen {
                    true => Some(events_loop.get_primary_monitor()),
                    false => None,
                });
            let context_builder = ContextBuilder::new().with_vsync(config.vsync);
            let display = Display::new(window_builder, context_builder, &events_loop)?;
            ::platform::window::init(display.gl_window().window(), &config);

            Backend::Display(display)
        } else {
            let context = HeadlessRendererBuilder::new(width, height).build()?;
            Backend::Headless(Headless::new(context)?)
        };

        debug!("{:?}", backend.as_ref().get_context().get_opengl_version_string());

        #[cfg_attr(rustfmt, rustfmt_skip)]
        let vertices = [
            Vertex { position: [-1.0, -1.0] },
            Vertex { position: [ 1.0, -1.0] },
            Vertex { position: [ 1.0,  1.0] },
            Vertex { position: [-1.0, -1.0] },
            Vertex { position: [ 1.0,  1.0] },
            Vertex { position: [-1.0,  1.0] },
        ];

        let vertex_buffer = VertexBuffer::new(backend.as_ref(), &vertices)?;
        let index_buffer = NoIndices(PrimitiveType::TrianglesList);
        let buffers = init_buffers(&config, backend.as_ref())?;

        // TODO: font should not be hardcoded
        let text_renderer = TextRenderer::new(backend.as_ref(), &config.font, config.font_size)?;

        Ok(Self {
            config: config,
            backend: backend,
            vertex_buffer: vertex_buffer,
            index_buffer: index_buffer,
            buffers: buffers,
            start_time: ::time::now(),
            text_renderer: text_renderer,
            fps_counter: FpsCounter::new(1.0),
        })
    }

    fn render(&mut self, pointer: [f32; 4]) -> Result<()> {
        let mut target = match self.backend {
            Backend::Display(ref facade) => facade.draw(),
            Backend::Headless(ref facade) => facade.draw(),
        };

        self.buffers["__default__"].borrow().render_to(
            &mut target,
            self.backend.as_ref(),
            &self.vertex_buffer,
            &self.index_buffer,
            ((::time::now() - self.start_time).num_nanoseconds().unwrap() as f32) / 1000_000_000.0 % 4096.0,
            pointer,
        )?;

        if self.config.fps {
            self.fps_counter.next_frame();

            self.text_renderer.draw_text(
                self.backend.as_ref(),
                &mut target,
                &format!("FPS: {:.1}", self.fps_counter.fps()),
                0.0,
                0.0,
                [1.0, 1.0, 1.0],
            )?;
        }

        target.finish()?;

        Ok(())
    }

    fn swap_buffers(&self) -> Result<()> {
        self.backend.as_ref().get_context().swap_buffers()?;
        Ok(())
    }

    fn reload(&mut self, config: &Config) -> Result<()> {
        info!("Reloading config");
        self.buffers = init_buffers(config, self.backend.as_ref())?;
        Ok(())
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        for buffer in self.buffers.values() {
            buffer
                .borrow_mut()
                .resize(self.backend.as_ref(), width, height)?;
        }
        Ok(())
    }

    fn render_to_file(&mut self, pointer: [f32; 4], path: &Path) -> Result<()> {
        self.buffers["__default__"].borrow().render_to_self(
            self.backend.as_ref(),
            &self.vertex_buffer,
            &self.index_buffer,
            ((::time::now() - self.start_time).num_nanoseconds().unwrap() as f32) / 1000_000_000.0 % 4096.0,
            pointer,
        )?;

        let raw: RawImage2d<u8> = self.buffers["__default__"].borrow().get_texture().read();
        let raw = RawImage2d::from_raw_rgba_reversed(&raw.data, (raw.width, raw.height));

        ::image::save_buffer(path, &raw.data, raw.width, raw.height, ::image::RGBA(8))?;

        Ok(())
    }
}
