//! A `Shader` contains a `Program` and renders it to an inner texture with inputs from
//! `Source`s and other `Shader` dependencies

use failure::{bail, ensure, Error, ResultExt};
use glium::{
    backend::Facade,
    implement_vertex,
    index::{NoIndices, PrimitiveType},
    program::ProgramCreationInput,
    texture::Texture2d,
    Program, Surface, VertexBuffer,
};
use std::{
    collections::HashMap,
    fs::File,
    io::{prelude::*, BufReader},
    rc::Rc,
};

use super::{Node, NodeInputs, NodeOutput};
use crate::{config::nodes::ShaderConfig, opengl::UniformsStorageVec};

/// Implementation of the vertex attributes for the vertex buffer
#[derive(Copy, Clone)]
pub struct Vertex {
    /// Position of the vertex in 2D space
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

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
    facade: Rc<dyn Facade>,
    /// A shader program which it uses for rendering
    program: Program,
    /// Vertex buffer
    vertex_buffer: VertexBuffer<Vertex>,
    /// Index buffer
    index_buffer: NoIndices,
}

impl ShaderNode {
    /// Create a new instance
    pub fn new(facade: &Rc<dyn Facade>, config: ShaderConfig) -> Result<Self, Error> {
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

        Ok(Self {
            facade: Rc::clone(facade),
            program,
            vertex_buffer: VertexBuffer::new(&**facade, &VERTICES)?,
            index_buffer: NoIndices(PrimitiveType::TrianglesList),
        })
    }
}

impl Node for ShaderNode {
    fn render(&mut self, inputs: &NodeInputs) -> Result<HashMap<String, NodeOutput>, Error> {
        if let NodeInputs::Shader { ref uniforms } = *inputs {
            let uniforms = {
                let mut storage = UniformsStorageVec::new();
                for (connection, uniform) in uniforms {
                    ensure!(
                        !connection.name.is_empty(),
                        "Connections for shader nodes must have a name"
                    );
                    let name = connection.name.clone();
                    match *uniform {
                        NodeOutput::Float(ref uniform) => storage.push(name, uniform.clone()),
                        NodeOutput::Float2(ref uniform) => storage.push(name, uniform.clone()),
                        NodeOutput::Color(ref uniform) | NodeOutput::Float4(ref uniform) => {
                            storage.push(name, uniform.clone())
                        }
                        NodeOutput::Texture2d(ref uniform) => storage.push(name, uniform.sampled()),
                        NodeOutput::Texture1d(ref uniform) => storage.push(name, uniform.sampled()),
                        _ => bail!("Wrong input type for `uniforms`"),
                    }
                }
                storage
            };

            let (width, height) = self.facade.get_context().get_framebuffer_dimensions();
            let texture = Rc::new(Texture2d::empty(&*self.facade, width, height)?);

            let mut surface = texture.as_surface();
            surface.clear_color(0.0, 0.0, 0.0, 1.0);
            surface.draw(
                &self.vertex_buffer,
                &self.index_buffer,
                &self.program,
                &uniforms,
                &Default::default(),
            )?;

            let mut outputs = HashMap::new();
            outputs.insert(
                "texture".to_string(),
                NodeOutput::Texture2d(Rc::clone(&texture)),
            );
            Ok(outputs)
        } else {
            bail!("Wrong input type for node");
        }
    }
}
