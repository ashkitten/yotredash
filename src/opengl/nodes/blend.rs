//! The blend node takes the output of other nodes and blends them to produce one output

use failure::Error;
use glium::backend::Facade;
use glium::draw_parameters::{Blend, DrawParameters};
use glium::index::{NoIndices, PrimitiveType};
use glium::program::ProgramCreationInput;
use glium::texture::{RawImage2d, Texture2d};
use glium::{Program, Surface, VertexBuffer};
use image;
use std::path::Path;
use std::rc::Rc;

use config::nodes::{BlendConfig, BlendOp};
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

    %INPUTS%

    void main() {
        vec2 uv = gl_FragCoord.xy / resolution;
        %BLENDS%
    }
";

/// A node that blends the output of other nodes
pub struct BlendNode {
    /// The name of the node
    name: String,
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
    /// List of input nodes
    inputs: Vec<String>,
}

impl BlendNode {
    /// Create a new instance
    pub fn new(facade: &Rc<Facade>, name: String, config: BlendConfig) -> Result<Self, Error> {
        let op_fmt = match config.operation {
            BlendOp::Min => "color = min(texture(%INPUT%, uv);",
            BlendOp::Max => "color = max(texture(%INPUT%, uv);",
            BlendOp::Add => "color += texture(%INPUT%, uv);",
            BlendOp::Sub => "color -= texture(%INPUT%, uv);",
        };

        let fragment = FRAGMENT
            .replace("%INPUTS%", {
                config
                    .inputs
                    .iter()
                    .map(|input| format!("uniform sampler2D {};", input))
                    .collect::<Vec<String>>()
                    .join("\n")
                    .as_str()
            })
            .replace("%BLENDS%", {
                let mut iter = config.inputs.iter();
                let first = iter.next().expect("Blend node needs at least one input");
                &format!(
                    "color = texture({}, uv);\n{}",
                    first,
                    iter.map(|input| op_fmt.replace("%INPUT%", input))
                        .collect::<Vec<String>>()
                        .join("\n")
                        .as_str()
                )
            });

        let program = {
            let input = ProgramCreationInput::SourceCode {
                vertex_shader: &VERTEX,
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
            name,
            facade: Rc::clone(facade),
            texture,
            program,
            vertex_buffer: VertexBuffer::new(&**facade, &VERTICES)?,
            index_buffer: NoIndices(PrimitiveType::TrianglesList),
            inputs: config.inputs,
        })
    }
}

impl Node for BlendNode {
    fn render(&mut self, inputs: &NodeInputs) -> Result<NodeOutputs, Error> {
        if let &NodeInputs::Blend { textures } = inputs {
            let resolution = (self.texture.width() as f32, self.texture.height() as f32);

            let uniforms = UniformsStorageVec::new();
            uniforms.push("resolution", resolution);
            for (name, texture) in textures {
                uniforms.push(name, texture.sampled());
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
        if let &NodeInputs::Blend { textures } = inputs {
            let resolution = self.facade.get_context().get_framebuffer_dimensions();
            let resolution = (resolution.0 as f32, resolution.1 as f32);

            let uniforms = UniformsStorageVec::new();
            uniforms.push("resolution", resolution);
            for (name, texture) in textures {
                uniforms.push(name, texture.sampled());
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
