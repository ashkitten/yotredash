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

/// Inputs for each node
pub enum NodeInputs {
    /// Inputs for info node
    Info,

    /// Inputs for output node
    Output {
        /// Texture to render to the screen
        texture: Rc<Texture2d>,
    },

    /// Inputs for image node
    Image,

    /// Inputs for shader node
    Shader {
        /// Node connections for uniforms as input for the shader program
        uniforms: HashMap<NodeConnection, NodeOutput>,
    },

    /// Inputs for blend node
    Blend {
        /// Textures to blend together
        textures: Vec<Rc<Texture2d>>,
    },

    /// Inputs for text node
    Text {
        /// Text to render
        text: Option<String>,
        /// Position to render at
        position: Option<[f32; 2]>,
        /// Color to render in
        color: Option<[f32; 4]>,
    },

    /// Inputs for FPS counter node
    Fps {
        /// Position to render at
        position: Option<[f32; 2]>,
        /// Color to render in
        color: Option<[f32; 4]>,
    },
}

/// Enum of possible output types for nodes
#[derive(Clone)]
pub enum NodeOutput {
    /// A color (RGBA)
    Color([f32; 4]),
    /// An f32
    Float(f32),
    /// An array of 2 f32 values
    Float2([f32; 2]),
    /// An array of 4 f32 values
    Float4([f32; 4]),
    /// A string
    Text(String),
    /// A 2D texture
    Texture2d(Rc<Texture2d>),
}

/// A `Node` is something that takes input and returns an output
pub trait Node {
    /// Does stuff and returns a `NodeOutputs`
    fn render(&mut self, inputs: &NodeInputs) -> Result<HashMap<String, NodeOutput>, Error>;
}
