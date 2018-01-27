//! A `Node` that takes a texture and draws it to the screen

use failure::Error;
use glium::backend::Facade;
use glium::draw_parameters::{Blend, DrawParameters};
use glium::program::{Program, ProgramCreationInput};
use glium::vertex::VertexBuffer;
use glium::index::{NoIndices, PrimitiveType};
use glium::Surface;
use std::collections::HashMap;
use std::rc::Rc;

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
    uniform sampler2D texture0;
    void main() {
        vec2 uv = gl_FragCoord.xy / resolution;
        color = texture(texture0, uv);
    }
";

/// A node that renders its input to the program output
pub struct OutputNode {
    /// The `Facade` it uses to work with OpenGL
    facade: Rc<Facade>,
    /// The shader program it uses to copy its input to the main output
    program: Program,
    /// Vertex buffer for the program
    vertex_buffer: VertexBuffer<Vertex>,
    /// Index buffer for the program
    index_buffer: NoIndices,
}

impl OutputNode {
    /// Create a new instance
    pub fn new(facade: &Rc<Facade>) -> Result<Self, Error> {
        let input = ProgramCreationInput::SourceCode {
            vertex_shader: VERTEX,
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            geometry_shader: None,
            fragment_shader: FRAGMENT,
            transform_feedback_varyings: None,
            outputs_srgb: true,
            uses_point_size: false,
        };

        Ok(Self {
            facade: Rc::clone(facade),
            program: Program::new(&**facade, input)?,
            vertex_buffer: VertexBuffer::new(&**facade, &VERTICES)?,
            index_buffer: NoIndices(PrimitiveType::TrianglesList),
        })
    }
}

impl Node for OutputNode {
    fn render(&mut self, inputs: &NodeInputs) -> Result<HashMap<String, NodeOutput>, Error> {
        if let NodeInputs::Output { ref texture } = *inputs {
            let (width, height) = self.facade.get_context().get_framebuffer_dimensions();

            let mut uniforms = UniformsStorageVec::new();
            uniforms.push("resolution", (width as f32, height as f32));
            uniforms.push("texture0", &**texture);

            let mut target = self.facade.draw();
            target.clear_color(0.0, 0.0, 0.0, 0.0);
            target
                .draw(
                    &self.vertex_buffer,
                    &self.index_buffer,
                    &self.program,
                    &uniforms,
                    &DrawParameters {
                        blend: Blend::alpha_blending(),
                        ..Default::default()
                    },
                )
                .unwrap(); // For some reason if we return this error, it panicks because finish() is never called
            target.finish()?;

            Ok(HashMap::new())
        } else {
            bail!("Wrong input type for node");
        }
    }
}
