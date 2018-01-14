//! A `Buffer` contains a `Program` and renders it to an inner texture with input uniforms from
//! `Source`s and other `Buffer` dependencies

use glium::{Program, Surface, VertexBuffer};
use glium::backend::Facade;
use glium::index::NoIndices;
use glium::program::ProgramCreationInput;
use glium::texture::{RawImage2d, Texture2d};
use owning_ref::OwningHandle;
use std::cell::RefCell;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::rc::Rc;
use failure::Error;
use failure::ResultExt;

use super::{MapAsUniform, UniformsStorageVec};
use super::renderer::Vertex;
use super::surface::OpenGLSurface;
use config::buffer_config::BufferConfig;
use source::Source;
use util::DerefInner;

/// The `Buffer` struct, containing most things it needs to render
pub struct Buffer {
    /// The name of the buffer, from the configuration
    name: String,
    /// The Facade it uses to work with the OpenGL context
    facade: Rc<Facade>,
    /// The inner texture which it renders to
    texture: Texture2d,
    /// A shader program which it uses for rendering
    program: Program,
    /// An array of `Source`s which it uses as input
    sources: Vec<(Rc<RefCell<Source>>, RefCell<OpenGLSurface>)>,
    /// An array of dependency buffers which must render themselves before this
    depends: Vec<Rc<RefCell<Buffer>>>,
    /// Whether or not the buffer should resize from its original dimensions
    resizeable: bool,
}

impl Buffer {
    /// Create a new instance using a `Facade` from the renderer, a configuration specific to that
    /// buffer, and an array of shared references to `Source`s
    pub fn new(
        name: &str,
        facade: Rc<Facade>,
        config: &BufferConfig,
        sources: Vec<Rc<RefCell<Source>>>,
    ) -> Result<Self, Error> {
        let vertex = config.path_to(&config.vertex);
        let fragment = config.path_to(&config.fragment);

        debug!("Using vertex shader: {}", vertex.to_str().unwrap());
        debug!("Using fragment shader: {}", fragment.to_str().unwrap());

        let file = File::open(vertex).context("Could not open vertex shader file")?;
        let mut buf_reader = BufReader::new(file);
        let mut vertex_source = String::new();
        buf_reader
            .read_to_string(&mut vertex_source)
            .context("Could not read vertex shader file")?;

        let file = File::open(fragment).context("Could not open fragment shader file")?;
        let mut buf_reader = BufReader::new(file);
        let mut fragment_source = String::new();
        buf_reader
            .read_to_string(&mut fragment_source)
            .context("Could not read fragment shader file")?;

        let input = ProgramCreationInput::SourceCode {
            vertex_shader: &vertex_source,
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            geometry_shader: None,
            fragment_shader: &fragment_source,
            transform_feedback_varyings: None,
            outputs_srgb: true,
            uses_point_size: false,
        };
        let program = Program::new(&*facade, input)?;

        let texture = Texture2d::empty(&*facade, config.width, config.height)?;

        let sources = sources
            .into_iter()
            .map(|source| {
                let frame = source.borrow().get_frame();
                let raw =
                    RawImage2d::from_raw_rgba_reversed(&frame.buffer, (frame.width, frame.height));
                let surface = OpenGLSurface::new(Rc::clone(&facade), raw).unwrap();
                (Rc::clone(&source), RefCell::new(surface))
            })
            .collect();

        Ok(Buffer {
            name: name.to_string(),
            facade: facade,
            texture: texture,
            program: program,
            sources: sources,
            depends: Vec::new(),
            resizeable: config.resizeable,
        })
    }

    /// Get the name of the buffer from the configuration
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Link dependency buffers
    pub fn link_depends(&mut self, depends: &mut Vec<Rc<RefCell<Buffer>>>) {
        self.depends.append(depends);
    }

    /// Render to a provided `Surface`
    pub fn render_to<S>(
        &self,
        surface: &mut S,
        vertex_buffer: &VertexBuffer<Vertex>,
        index_buffer: &NoIndices,
        time: f32,
        pointer: [f32; 4],
    ) -> Result<(), Error>
    where
        S: Surface,
    {
        surface.clear_color(0.0, 0.0, 0.0, 1.0);

        let mut uniforms = UniformsStorageVec::new();

        uniforms.push(
            "resolution",
            (
                surface.get_dimensions().0 as f32,
                surface.get_dimensions().1 as f32,
            ),
        );

        uniforms.push("time", time);

        uniforms.push(
            "pointer",
            [
                pointer[0],
                surface.get_dimensions().1 as f32 - pointer[1],
                pointer[2],
                surface.get_dimensions().1 as f32 - pointer[3],
            ],
        );

        for source in &self.sources {
            if source.0.borrow_mut().update() {
                use std::borrow::BorrowMut;

                let mut surface_ref = source.1.borrow_mut();
                source.0.borrow().write_frame((*surface_ref).borrow_mut())?;
            }

            let surface = OwningHandle::new(&source.1);
            let texture = OwningHandle::new_with_fn(surface, |s| unsafe {
                DerefInner((*s).ref_texture().sampled())
            });
            let sampled = MapAsUniform(texture, |t| &**t);
            uniforms.push(source.0.borrow().get_name().to_string(), sampled);
        }

        for buffer in &self.depends {
            buffer
                .borrow()
                .render_to_self(vertex_buffer, index_buffer, time, pointer)?;

            let name = buffer.borrow().get_name().to_string();

            let buffer = OwningHandle::new(&**buffer);
            let texture = OwningHandle::new_with_fn(buffer, |b| unsafe {
                DerefInner((*b).texture.sampled())
            });
            let sampled = MapAsUniform(texture, |t| &**t);
            uniforms.push(name, sampled);
        }

        surface.draw(
            vertex_buffer,
            index_buffer,
            &self.program,
            &uniforms,
            &Default::default(),
        )?;

        Ok(())
    }

    /// Render to the internal texture
    pub fn render_to_self(
        &self,
        vertex_buffer: &VertexBuffer<Vertex>,
        index_buffer: &NoIndices,
        time: f32,
        pointer: [f32; 4],
    ) -> Result<(), Error> {
        self.render_to(
            &mut self.texture.as_surface(),
            vertex_buffer,
            index_buffer,
            time,
            pointer,
        )?;
        Ok(())
    }

    /// Resize the internal texture
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), Error> {
        if self.resizeable {
            self.texture = Texture2d::empty(&*self.facade, width, height)?
        }
        Ok(())
    }

    /// Get a reference to the internal texture
    pub fn get_texture(&self) -> &Texture2d {
        &self.texture
    }
}
