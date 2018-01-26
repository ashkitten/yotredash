//! A `Shader` contains a `Program` and renders it to an inner texture with inputs from
//! `Source`s and other `Shader` dependencies

use failure::Error;
use failure::ResultExt;
use glium::backend::Facade;
use glium::draw_parameters::{Blend, DrawParameters};
use glium::index::{NoIndices, PrimitiveType};
use glium::program::ProgramCreationInput;
use glium::texture::{RawImage2d, Texture2d};
use glium::{Program, Surface, VertexBuffer};
use image;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::path::Path;
use std::rc::Rc;

use config::nodes::{NodeParameter, ShaderConfig};
use opengl::{UniformsStorageVec, Vertex};
use super::{Node, NodeInputs, NodeOutputs};

#[cfg_attr(rustfmt, rustfmt_skip)]
const VERTICES: [Vertex; 6] = [
    Vertex { position: [-1.0, -1.0] },
    Vertex { position: [ 1.0, -1.0] },
    Vertex { position: [ 1.0,  1.0] },
    Vertex { position: [-1.0, -1.0] },
    Vertex { position: [ 1.0,  1.0] },
    Vertex { position: [-1.0,  1.0] },
];

/// A node that renders a shader program
pub struct ShaderNode {
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
    /// List of input nodes
    textures: Vec<String>,
}

impl ShaderNode {
    /// Create a new instance
    pub fn new(facade: &Rc<Facade>, config: ShaderConfig) -> Result<Self, Error> {
        let file = File::open(config.vertex).context("Could not open vertex shader file")?;
        let mut buf_reader = BufReader::new(file);
        let mut vertex_source = String::new();
        buf_reader
            .read_to_string(&mut vertex_source)
            .context("Could not read vertex shader file")?;

        let file = File::open(config.fragment).context("Could not open fragment shader file")?;
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

        let (width, height) = facade.get_context().get_framebuffer_dimensions();
        let texture = Rc::new(Texture2d::empty(&**facade, width, height)?);

        // TODO: Handle Static case
        let textures: Vec<String> = config
            .textures
            .iter()
            .map(|texture| match texture {
                &NodeParameter::NodeConnection { ref node } => node.to_string(),
                &NodeParameter::Static(_) => unimplemented!(),
            })
            .collect();

        Ok(Self {
            facade: Rc::clone(facade),
            texture,
            program,
            vertex_buffer: VertexBuffer::new(&**facade, &VERTICES)?,
            index_buffer: NoIndices(PrimitiveType::TrianglesList),
            textures,
        })
    }
}

impl Node for ShaderNode {
    fn render(&mut self, inputs: &NodeInputs) -> Result<NodeOutputs, Error> {
        if let &NodeInputs::Shader {
            ref time,
            ref pointer,
            ref textures,
        } = inputs
        {
            let resolution = (self.texture.width() as f32, self.texture.height() as f32);

            let mut uniforms = UniformsStorageVec::new();
            uniforms.push("resolution", resolution);
            uniforms.push("time", time.clone());
            uniforms.push("pointer", pointer.clone());
            for (name, texture) in textures {
                uniforms.push(name.to_string(), texture.sampled());
            }

            let mut surface = self.texture.as_surface();
            surface.clear_color(0.0, 0.0, 0.0, 0.0);
            surface.draw(
                &self.vertex_buffer,
                &self.index_buffer,
                &self.program,
                &uniforms,
                &DrawParameters {
                    blend: Blend::alpha_blending(),
                    ..Default::default()
                },
            )?;

            Ok(NodeOutputs::Texture2d(Rc::clone(&self.texture)))
        } else {
            bail!("Wrong input type for node");
        }
    }

    fn present(&mut self, inputs: &NodeInputs) -> Result<(), Error> {
        if let &NodeInputs::Shader {
            ref time,
            ref pointer,
            ref textures,
        } = inputs
        {
            let resolution = self.facade.get_context().get_framebuffer_dimensions();
            let resolution = (resolution.0 as f32, resolution.1 as f32);

            let mut uniforms = UniformsStorageVec::new();
            uniforms.push("resolution", resolution);
            uniforms.push("time", time.clone());
            uniforms.push("pointer", pointer.clone());
            for (name, texture) in textures {
                uniforms.push(name.to_string(), texture.sampled());
            }

            let mut target = self.facade.draw();
            target.clear_color(0.0, 0.0, 0.0, 0.0);
            target.draw(
                &self.vertex_buffer,
                &self.index_buffer,
                &self.program,
                &uniforms,
                &DrawParameters {
                    blend: Blend::alpha_blending(),
                    ..Default::default()
                },
            )?;
            target.finish()?;

            Ok(())
        } else {
            bail!("Wrong input type for node");
        }
    }

    fn render_to_file(&mut self, inputs: &NodeInputs, path: &Path) -> Result<(), Error> {
        self.render(inputs)?;

        let raw: RawImage2d<u8> = self.texture.read();
        let raw = RawImage2d::from_raw_rgba_reversed(&raw.data, (raw.width, raw.height));

        image::save_buffer(path, &raw.data, raw.width, raw.height, image::RGBA(8))?;

        Ok(())
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<(), Error> {
        self.texture = Rc::new(Texture2d::empty(&*self.facade, width, height)?);

        Ok(())
    }
}
