//! A `Shader` contains a `Program` and renders it to an inner texture with inputs from
//! `Source`s and other `Shader` dependencies

use failure::Error;
use failure::ResultExt;
use glium::backend::Facade;
use glium::index::{NoIndices, PrimitiveType};
use glium::program::ProgramCreationInput;
use glium::texture::Texture2d;
use glium::{Program, Surface, VertexBuffer};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::rc::Rc;
use std::sync::mpsc::Receiver;

use config::nodes::ShaderConfig;
use event::RendererEvent;
use opengl::UniformsStorageVec;
use super::{Node, NodeInputs, NodeOutput};

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
    facade: Rc<Facade>,
    /// The inner texture which it renders to
    texture: Rc<Texture2d>,
    /// A shader program which it uses for rendering
    program: Program,
    /// Vertex buffer
    vertex_buffer: VertexBuffer<Vertex>,
    /// Index buffer
    index_buffer: NoIndices,
    /// Receiver for events
    receiver: Receiver<RendererEvent>,
}

impl ShaderNode {
    /// Create a new instance
    pub fn new(
        facade: &Rc<Facade>,
        config: ShaderConfig,
        receiver: Receiver<RendererEvent>,
    ) -> Result<Self, Error> {
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

        Ok(Self {
            facade: Rc::clone(facade),
            texture,
            program,
            vertex_buffer: VertexBuffer::new(&**facade, &VERTICES)?,
            index_buffer: NoIndices(PrimitiveType::TrianglesList),
            receiver,
        })
    }
}

impl Node for ShaderNode {
    fn render(&mut self, inputs: &NodeInputs) -> Result<HashMap<String, NodeOutput>, Error> {
        if let Ok(event) = self.receiver.try_recv() {
            match event {
                RendererEvent::Resize(width, height) => {
                    self.texture = Rc::new(Texture2d::empty(&*self.facade, width, height)?);
                }
                _ => (),
            }
        }

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
                        _ => bail!("Wrong input type for `uniforms`"),
                    }
                }
                storage
            };

            let mut surface = self.texture.as_surface();
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
                NodeOutput::Texture2d(Rc::clone(&self.texture)),
            );
            Ok(outputs)
        } else {
            bail!("Wrong input type for node");
        }
    }
}
