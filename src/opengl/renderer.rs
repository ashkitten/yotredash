//! An implementation of `Renderer` using OpenGL

use glium::VertexBuffer;
use glium::backend::Facade;
use glium::backend::glutin::Display;
use glium::backend::glutin::headless::Headless;
use glium::glutin::{ContextBuilder, GlProfile, HeadlessRendererBuilder, WindowBuilder};
use glium::index::{NoIndices, PrimitiveType};
use glium::texture::RawImage2d;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use winit::EventsLoop;

use super::buffer::Buffer;
use super::text_renderer::TextRenderer;
use Renderer;
use config::Config;
use errors::*;
use source::{ImageSource, Source};

/// Implementation of the vertex attributes for the vertex buffer
#[derive(Copy, Clone)]
pub struct Vertex {
    /// Position of the vertex in 2D space
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

/// An implementation of a `Renderer` which uses OpenGL
pub struct OpenGLRenderer {
    /// The configuration from file
    config: Config,
    /// The facade it uses to render
    facade: Rc<Facade>,
    /// The vertex buffer, so we don't have to recreate it
    vertex_buffer: VertexBuffer<Vertex>,
    /// The index buffer, so we don't have to recreate it
    index_buffer: NoIndices,
    /// A map of names to Buffer references
    buffers: HashMap<String, Rc<RefCell<Buffer>>>,
    /// An instance of the text renderer
    text_renderer: TextRenderer,
}

fn init_buffers(
    config: &Config,
    facade: &Rc<Facade>,
) -> Result<HashMap<String, Rc<RefCell<Buffer>>>> {
    let mut sources = HashMap::new();

    for (name, sconfig) in &config.sources {
        sources.insert(
            name.to_string(),
            match sconfig.kind.as_str() {
                "image" => Rc::new(RefCell::new(ImageSource::new(
                    name,
                    &config.path_to(&sconfig.path),
                )?)),
                _ => bail!("Unsupported kind of source"),
            }: Rc<RefCell<Source>>,
        );
    }

    let mut buffers = HashMap::new();

    for (name, bconfig) in &config.buffers {
        buffers.insert(
            name.to_string(),
            Rc::new(RefCell::new(Buffer::new(
                name,
                Rc::clone(facade),
                bconfig,
                bconfig
                    .sources
                    .iter()
                    .map(|name| Rc::clone(&sources[name]))
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
        let facade: Rc<Facade> = if !config.headless {
            let window_builder = WindowBuilder::new()
                .with_title("yotredash")
                .with_maximized(config.maximize)
                .with_fullscreen(if config.fullscreen {
                    Some(events_loop.get_primary_monitor())
                } else {
                    None
                });
            let context_builder = ContextBuilder::new().with_vsync(config.vsync);
            let display = Display::new(window_builder, context_builder, events_loop)?;
            ::platform::window::init(display.gl_window().window(), &config);

            Rc::new(display)
        } else {
            let context = HeadlessRendererBuilder::new(width, height)
                .with_gl_profile(GlProfile::Core)
                .build()?;
            Rc::new(Headless::new(context)?)
        };

        info!("{:?}", facade.get_context().get_opengl_version_string());

        #[cfg_attr(rustfmt, rustfmt_skip)]
        let vertices = [
            Vertex { position: [-1.0, -1.0] },
            Vertex { position: [ 1.0, -1.0] },
            Vertex { position: [ 1.0,  1.0] },
            Vertex { position: [-1.0, -1.0] },
            Vertex { position: [ 1.0,  1.0] },
            Vertex { position: [-1.0,  1.0] },
        ];

        let vertex_buffer = VertexBuffer::new(&*facade, &vertices)?;
        let index_buffer = NoIndices(PrimitiveType::TrianglesList);

        let buffers = init_buffers(&config, &facade)?;

        let text_renderer = TextRenderer::new(Rc::clone(&facade), &config.font, config.font_size)?;

        Ok(Self {
            config: config,
            facade: facade,
            vertex_buffer: vertex_buffer,
            index_buffer: index_buffer,
            buffers: buffers,
            text_renderer: text_renderer,
        })
    }

    fn render(&mut self, time: ::time::Duration, pointer: [f32; 4], fps: f32) -> Result<()> {
        let mut target = self.facade.draw();

        self.buffers["__default__"].borrow().render_to(
            &mut target,
            &self.vertex_buffer,
            &self.index_buffer,
            (time.num_nanoseconds().unwrap() as f32) / 1000_000_000.0 % 4096.0,
            pointer,
        )?;

        if self.config.fps {
            self.text_renderer.draw_text(
                &mut target,
                &format!("FPS: {:.1}", fps),
                0.0,
                0.0,
                [1.0, 1.0, 1.0],
            )?;
        }

        target.finish()?;

        Ok(())
    }

    fn swap_buffers(&self) -> Result<()> {
        self.facade.get_context().swap_buffers()?;
        Ok(())
    }

    fn reload(&mut self, config: &Config) -> Result<()> {
        info!("Reloading config");
        self.buffers = init_buffers(config, &self.facade)?;
        Ok(())
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        for buffer in self.buffers.values() {
            buffer.borrow_mut().resize(width, height)?;
        }
        Ok(())
    }

    fn render_to_file(
        &mut self,
        time: ::time::Duration,
        pointer: [f32; 4],
        fps: f32,
        path: &Path,
    ) -> Result<()> {
        self.buffers["__default__"].borrow().render_to_self(
            &self.vertex_buffer,
            &self.index_buffer,
            (time.num_nanoseconds().unwrap() as f32) / 1000_000_000.0 % 4096.0,
            pointer,
        )?;

        let buffer = self.buffers["__default__"].borrow();
        let texture = buffer.get_texture();
        let mut target = texture.as_surface();

        if self.config.fps {
            self.text_renderer.draw_text(
                &mut target,
                &format!("FPS: {:.1}", fps),
                0.0,
                0.0,
                [1.0, 1.0, 1.0],
            )?;
        }

        let raw: RawImage2d<u8> = texture.read();
        let raw = RawImage2d::from_raw_rgba_reversed(&raw.data, (raw.width, raw.height));

        ::image::save_buffer(path, &raw.data, raw.width, raw.height, ::image::RGBA(8))?;

        Ok(())
    }
}
