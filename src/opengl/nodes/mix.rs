use failure::Error;
use glium::backend::Facade;
use glium::index::{NoIndices, PrimitiveType};
use glium::program::ProgramCreationInput;
use glium::texture::{RawImage2d, Texture2d};
use glium::{Program, Surface, VertexBuffer};
use image;
use owning_ref::OwningHandle;
use std::path::Path;
use std::rc::Rc;

use opengl::{UniformsStorageVec, MapAsUniform, Vertex};
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
        %MIXING%
    }
";

/// A node that mixes other nodes
pub struct MixNode {
    /// The name of the node
    name: String,
    /// The Facade it uses to work with the OpenGL context
    facade: Rc<Facade>,
    /// The inner texture it renders to
    texture: Rc<Texture2d>,
    /// Shader program used to mix the inputs
    program: Program,
    /// Vertex buffer for the shader
    vertex_buffer: VertexBuffer<Vertex>,
    /// Index buffer for the shader
    index_buffer: NoIndices,
}

impl MixNode {
    /// Create a new instance
    pub fn new(
        facade: &Rc<Facade>,
        name: String,
        inputs: Vec<(String, f32)>,
    ) -> Result<Self, Error> {
        debug!("New MixNode: {}, inputs: {:?}", name, inputs);

        let fragment = FRAGMENT
            .replace("%INPUTS%", {
                inputs
                    .iter()
                    .map(|input| format!("uniform sampler2D {};", input.0))
                    .collect::<Vec<String>>()
                    .join("\n")
                    .as_str()
            })
            .replace("%MIXING%", {
                let mut iter = inputs.iter();
                &format!(
                    "color = texture({}, uv);\n{}",
                    iter.next().expect("Mix node have at least one input").0,
                    iter.map(|input| format!(
                        "color = mix(color, texture({}, uv), {});",
                        input.0, input.1
                    )).collect::<Vec<String>>()
                        .join("\n")
                        .as_str()
                )
            });

        println!("{}", fragment);

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
        })
    }
}

impl Node for MixNode {
    fn render(&mut self, uniforms: &mut UniformsStorageVec) -> Result<(), Error> {
        let mut surface = self.texture.as_surface();

        surface.clear_color(0.0, 0.0, 0.0, 1.0);
        surface.draw(
            &self.vertex_buffer,
            &self.index_buffer,
            &self.program,
            uniforms,
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
        let mut target = self.facade.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);
        target.draw(
            &self.vertex_buffer,
            &self.index_buffer,
            &self.program,
            uniforms,
            &Default::default(),
        ).unwrap(); // For some reason if we return this error, it panicks because finish() is never called
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
