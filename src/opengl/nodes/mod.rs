//! Nodes are the basic building blocks for the renderer.
// TODO: expand documentation and add examples

#[cfg(feature = "image-src")]
pub mod image;

pub mod blend;
pub mod fps;
pub mod info;
pub mod output;
pub mod shader;
pub mod text;

use failure::Error;
use glium::texture::Texture2d;
use std::collections::HashMap;
use std::rc::Rc;

#[cfg(feature = "image-src")]
pub use self::image::ImageNode;

use config::nodes::NodeConnection;
pub use self::blend::BlendNode;
pub use self::fps::FpsNode;
pub use self::output::OutputNode;
pub use self::shader::ShaderNode;
pub use self::text::TextNode;
pub use self::info::InfoNode;

pub enum NodeInputs {
    Info,
    Output {
        texture: Rc<Texture2d>,
    },
    Image,
    Shader {
        uniforms: HashMap<NodeConnection, NodeOutput>,
    },
    Blend {
        textures: Vec<Rc<Texture2d>>,
    },
    Text {
        text: Option<String>,
        position: Option<[f32; 2]>,
        color: Option<[f32; 4]>,
    },
    Fps {
        position: Option<[f32; 2]>,
        color: Option<[f32; 4]>,
    },
}

#[derive(Clone)]
pub enum NodeOutput {
    Color([f32; 4]),
    Float(f32),
    Float2([f32; 2]),
    Float4([f32; 4]),
    Text(String),
    Texture2d(Rc<Texture2d>),
}

/// A `Node` is something that takes input and returns an output
pub trait Node {
    /// Does stuff and returns a `NodeOutputs`
    fn render(&mut self, inputs: &NodeInputs) -> Result<HashMap<String, NodeOutput>, Error>;
    /// Called on a window resize event
    fn resize(&mut self, width: u32, height: u32) -> Result<(), Error>;
}
