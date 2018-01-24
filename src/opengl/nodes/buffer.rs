//! A `Buffer` contains a `Program` and renders it to an inner texture with input uniforms from
//! `Source`s and other `Buffer` dependencies

use failure::Error;
use failure::ResultExt;
use glium::backend::Facade;
use glium::index::{NoIndices, PrimitiveType};
use glium::program::ProgramCreationInput;
use glium::texture::{RawImage2d, Texture2d};
use glium::{Program, Surface, VertexBuffer};
use image;
use owning_ref::OwningHandle;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::path::Path;
use std::rc::Rc;

use opengl::{MapAsUniform, UniformsStorageVec, Vertex};
use super::Node;
use util::DerefInner;

#[cfg_attr(rustfmt, rustfmt_skip)]
const VERTICES: [Vertex; 6] = [
    Vertex { position: [-1.0, -1.0] },
    Vertex { position: [ 1.0, -1.0] },
    Vertex { position: [ 1.0,  1.0] },
    Vertex { position: [-1.0, -1.0] },
    Vertex { position: [ 1.0,  1.0] },
    Vertex { position: [-1.0,  1.0] },
];

/// The `Buffer` struct, containing most things it needs to render
pub struct BufferNode {
    /// The name of the node
    name: String,
    /// The Facade it uses to work with the OpenGL context
    facade: Rc<Facade>,
    /// The inner texture which it renders to
    texture: Rc<Texture2d>,
    /// A shader program which it uses for rendering
    program: Program,
    /// Vertex buffer
    vertex_buffer: VertexBuffer<Vertex>,
    /// Index buffer
    index_buffer: NoIndices,
}

impl BufferNode {
    /// Create a new instance
    pub fn new(
        facade: &Rc<Facade>,
        name: String,
        vertex: &Path,
        fragment: &Path,
    ) -> Result<Self, Error> {
        let (width, height) = facade.get_context().get_framebuffer_dimensions();

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

        let program = Program::new(&**facade, input)?;

        let texture = Rc::new(Texture2d::empty(&**facade, width, height)?);

        let vertex_buffer = VertexBuffer::new(&**facade, &VERTICES)?;
        let index_buffer = NoIndices(PrimitiveType::TrianglesList);

        Ok(Self {
            name,
            facade: Rc::clone(facade),
            texture,
            program,
            vertex_buffer,
            index_buffer,
        })
    }
}

impl Node for BufferNode {
    fn render(&mut self, uniforms: &mut UniformsStorageVec) -> Result<(), Error> {
        let mut surface = self.texture.as_surface();

        let mut input = uniforms.clone();
        input.push(
            "resolution",
            (
                surface.get_dimensions().0 as f32,
                surface.get_dimensions().1 as f32,
            ),
        );

        surface.clear_color(0.0, 0.0, 0.0, 1.0);
        surface.draw(
            &self.vertex_buffer,
            &self.index_buffer,
            &self.program,
            &input,
            &Default::default(),
        )?;

        let sampled = OwningHandle::new_with_fn(self.texture.clone(), |t| unsafe {
            DerefInner((*t).sampled())
        });
        let sampled = MapAsUniform(sampled, |s| &**s);

        uniforms.push(self.name.clone(), sampled);

        Ok(())
    }

    fn present(&mut self, uniforms: &mut UniformsStorageVec) -> Result<(), Error> {
        let mut input = uniforms.clone();
        input.push("resolution", {
            let (width, height) = self.facade.get_context().get_framebuffer_dimensions();
            (width as f32, height as f32)
        });

        let mut target = self.facade.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);
        target.draw(
            &self.vertex_buffer,
            &self.index_buffer,
            &self.program,
            &input,
            &Default::default(),
        )?;
        target.finish()?;

        Ok(())
    }

    fn render_to_file(
        &mut self,
        uniforms: &mut UniformsStorageVec,
        path: &Path,
    ) -> Result<(), Error> {
        self.render(uniforms)?;

        let raw: RawImage2d<u8> = self.texture.read();
        let raw = RawImage2d::from_raw_rgba_reversed(&raw.data, (raw.width, raw.height));

        image::save_buffer(path, &raw.data, raw.width, raw.height, ::image::RGBA(8))?;

        Ok(())
    }
}
