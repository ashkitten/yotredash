//! The blend node takes the output of other nodes and blends them to produce one output

use failure::Error;
use glium::backend::Facade;
use glium::index::{NoIndices, PrimitiveType};
use glium::program::ProgramCreationInput;
use glium::texture::Texture2d;
use glium::{Program, Surface, VertexBuffer};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc::Receiver;

use super::{Node, NodeInputs, NodeOutput};
use config::nodes::{BlendConfig, BlendOp};
use event::RendererEvent;
use opengl::UniformsStorageVec;

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

const VERTEX: &str = "
    #version 140

    in vec2 position;

    void main() {
        gl_Position = vec4(position, 0.0, 1.0);
    }
";

const FRAGMENT: &str = "
    #version 140

    out vec4 color;

    uniform vec2 resolution;

    %TEXTURES%

    void main() {
        vec2 uv = gl_FragCoord.xy / resolution;
        %BLENDS%
    }
";

/// A node that blends the output of other nodes
pub struct BlendNode {
    /// The Facade it uses to work with the OpenGL context
    facade: Rc<Facade>,
    /// The inner texture it renders to
    texture: Rc<Texture2d>,
    /// Shader program used to blend the inputs
    program: Program,
    /// Vertex buffer for the shader
    vertex_buffer: VertexBuffer<Vertex>,
    /// Index buffer for the shader
    index_buffer: NoIndices,
    /// Receiver for events
    receiver: Receiver<RendererEvent>,
}

impl BlendNode {
    /// Create a new instance
    pub fn new(
        facade: &Rc<Facade>,
        config: &BlendConfig,
        receiver: Receiver<RendererEvent>,
    ) -> Result<Self, Error> {
        let op_fmt = match config.operation {
            BlendOp::Min => "color = min(texture(%INPUT%, uv);",
            BlendOp::Max => "color = max(texture(%INPUT%, uv);",
            BlendOp::Add => "color += texture(%INPUT%, uv);",
            BlendOp::Sub => "color -= texture(%INPUT%, uv);",
        };

        let fragment = FRAGMENT
            .replace("%TEXTURES%", {
                (0..config.textures.len())
                    .map(|i| format!("uniform sampler2D texture_{};", i))
                    .collect::<Vec<String>>()
                    .join("\n")
                    .as_str()
            })
            .replace("%BLENDS%", {
                let mut iter = (0..config.textures.len()).map(|i| format!("texture_{}", i));
                &format!(
                    "color = texture({}, uv);\n{}",
                    iter.next().expect("Blend node needs at least one input"),
                    iter.map(|name| op_fmt.replace("%INPUT%", &name))
                        .collect::<Vec<String>>()
                        .join("\n")
                        .as_str()
                )
            });

        let program = {
            let input = ProgramCreationInput::SourceCode {
                vertex_shader: VERTEX,
                tessellation_control_shader: None,
                tessellation_evaluation_shader: None,
                geometry_shader: None,
                fragment_shader: &fragment,
                transform_feedback_varyings: None,
                outputs_srgb: true,
                uses_point_size: false,
            };
            Program::new(&**facade, input)?
        };

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

impl Node for BlendNode {
    fn render(&mut self, inputs: &NodeInputs) -> Result<HashMap<String, NodeOutput>, Error> {
        if let Ok(event) = self.receiver.try_recv() {
            match event {
                RendererEvent::Resize(width, height) => {
                    self.texture = Rc::new(Texture2d::empty(&*self.facade, width, height)?);
                }
                _ => (),
            }
        }

        if let NodeInputs::Blend { ref textures } = *inputs {
            let resolution = (self.texture.width() as f32, self.texture.height() as f32);

            let mut uniforms = UniformsStorageVec::new();
            uniforms.push("resolution", resolution);
            for (i, texture) in textures.iter().enumerate() {
                uniforms.push(format!("texture_{}", i), texture.sampled());
            }

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
