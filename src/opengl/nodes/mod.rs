//! Nodes are the basic building blocks for the renderer.
// TODO: expand documentation and add examples

pub mod audio;
pub mod blend;
pub mod feedback;
pub mod fps;
pub mod image;
pub mod info;
pub mod output;
pub mod shader;
pub mod text;

use failure::Error;
use glium::texture::{Texture1d, Texture2d};
use std::{collections::HashMap, rc::Rc};

pub use self::{
    audio::AudioNode, blend::BlendNode, feedback::FeedbackNode, fps::FpsNode, image::ImageNode,
    info::InfoNode, output::OutputNode, shader::ShaderNode, text::TextNode,
};
use crate::config::nodes::NodeConnection;

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

    /// Inputs for audio node
    Audio,

    /// Inputs for feedback node (unused because we have to special-case it somewhere else)
    Feedback,
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
    /// A 1D texture
    Texture1d(Rc<Texture1d>),
}

/// An enum of all node types
pub enum NodeType {
    /// Info node
    Info(InfoNode),
    /// Output node
    Output(OutputNode),
    /// Image node
    Image(ImageNode),
    /// Shader node
    Shader(ShaderNode),
    /// Blend node
    Blend(BlendNode),
    /// Text node
    Text(TextNode),
    /// Fps node
    Fps(FpsNode),
    /// Audio node
    Audio(AudioNode),
    /// Feedback node
    Feedback(FeedbackNode),
}

impl Node for NodeType {
    fn render(&mut self, inputs: &NodeInputs) -> Result<HashMap<String, NodeOutput>, Error> {
        use self::NodeType::*;
        match self {
            &mut Info(ref mut node) => node.render(inputs),
            &mut Output(ref mut node) => node.render(inputs),
            &mut Image(ref mut node) => node.render(inputs),
            &mut Shader(ref mut node) => node.render(inputs),
            &mut Blend(ref mut node) => node.render(inputs),
            &mut Text(ref mut node) => node.render(inputs),
            &mut Fps(ref mut node) => node.render(inputs),
            &mut Audio(ref mut node) => node.render(inputs),
            &mut Feedback(ref mut node) => node.render(inputs),
        }
    }
}

/// A `Node` is something that takes input and returns an output
pub trait Node {
    /// Does stuff and returns a `NodeOutputs`
    fn render(&mut self, inputs: &NodeInputs) -> Result<HashMap<String, NodeOutput>, Error>;
}
