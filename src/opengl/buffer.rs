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

use super::{MapAsUniform, UniformsStorageVec};
use super::renderer::Vertex;
use config::buffer_config::BufferConfig;
use errors::*;
use source::Source;
use util::DerefInner;

pub struct Buffer {
    texture: Texture2d,
    program: Program,
    sources: Vec<(Rc<RefCell<Source>>, RefCell<Texture2d>)>,
    depends: Vec<Rc<RefCell<Buffer>>>,
    resizeable: bool,
}

impl Buffer {
    pub fn new(facade: &Facade, config: &BufferConfig, sources: Vec<Rc<RefCell<Source>>>) -> Result<Self> {
        info!("Using vertex shader: {}", config.vertex);
        info!("Using fragment shader: {}", config.fragment);

        let file = File::open(config.path_to(&config.vertex)).chain_err(|| "Could not open vertex shader file")?;
        let mut buf_reader = BufReader::new(file);
        let mut vertex_source = String::new();
        buf_reader
            .read_to_string(&mut vertex_source)
            .chain_err(|| "Could not read vertex shader file")?;

        let file = File::open(config.path_to(&config.fragment)).chain_err(|| "Could not open fragment shader file")?;
        let mut buf_reader = BufReader::new(file);
        let mut fragment_source = String::new();
        buf_reader
            .read_to_string(&mut fragment_source)
            .chain_err(|| "Could not read fragment shader file")?;

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
        let program = Program::new(facade, input)?;

        let texture = Texture2d::empty(facade, config.width, config.height)?;

        let sources = sources
            .into_iter()
            .map(|source| {
                let frame = source.borrow().get_frame();
                let raw = RawImage2d::from_raw_rgba_reversed(&frame.buffer, (frame.width, frame.height));
                (
                    source.clone(),
                    RefCell::new(Texture2d::new(facade, raw).unwrap()),
                )
            })
            .collect();

        Ok(Buffer {
            texture: texture,
            program: program,
            sources: sources,
            depends: Vec::new(),
            resizeable: config.resizeable,
        })
    }

    pub fn link_depends(&mut self, depends: &mut Vec<Rc<RefCell<Buffer>>>) {
        self.depends.append(depends);
    }

    pub fn render_to<S>(
        &self, surface: &mut S, facade: &Facade, vertex_buffer: &VertexBuffer<Vertex>, index_buffer: &NoIndices,
        time: f32, pointer: [f32; 4],
    ) -> Result<()>
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

        for (i, source) in self.sources.iter().enumerate() {
            if source.0.borrow_mut().update() {
                let frame = source.0.borrow().get_frame();
                let raw = RawImage2d::from_raw_rgba_reversed(&frame.buffer, (frame.width, frame.height));
                source.1.replace(Texture2d::new(facade, raw)?);
            }

            let texture = OwningHandle::new(&source.1);
            let texture = OwningHandle::new_with_fn(texture, |t| unsafe { DerefInner((*t).sampled()) });
            let texture = MapAsUniform(texture, |t| &**t);
            uniforms.push(format!("texture{}", i), texture);
        }

        for (i, buffer) in self.depends.iter().enumerate() {
            buffer
                .borrow()
                .render_to_self(facade, vertex_buffer, index_buffer, time, pointer)?;

            let buffer = OwningHandle::new(&**buffer);
            let texture = OwningHandle::new_with_fn(buffer, |b| unsafe {
                DerefInner((*b).texture.sampled())
            });
            let texture = MapAsUniform(texture, |t| &**t);
            uniforms.push(format!("buffer{}", i), texture);
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

    pub fn render_to_self(
        &self, facade: &Facade, vertex_buffer: &VertexBuffer<Vertex>, index_buffer: &NoIndices, time: f32,
        pointer: [f32; 4],
    ) -> Result<()> {
        self.render_to(
            &mut self.texture.as_surface(),
            facade,
            vertex_buffer,
            index_buffer,
            time,
            pointer,
        )?;
        Ok(())
    }

    pub fn resize(&mut self, facade: &Facade, width: u32, height: u32) -> Result<()> {
        if self.resizeable {
            self.texture = Texture2d::empty(facade, width, height)?
        }
        Ok(())
    }

    pub fn get_texture(&self) -> &Texture2d {
        &self.texture
    }
}
