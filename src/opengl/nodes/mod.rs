//! Nodes are the basic building blocks for the renderer.
// TODO: expand documentation and add examples

#[cfg(feature = "image-src")]
pub mod image;

pub mod blend;
pub mod fps;
pub mod shader;
pub mod text;

use failure::Error;
use glium::texture::Texture2d;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

#[cfg(feature = "image-src")]
pub use self::image::ImageNode;

pub use self::blend::BlendNode;
pub use self::fps::FpsNode;
pub use self::shader::ShaderNode;
pub use self::text::TextNode;

#[derive(Debug)]
pub enum NodeInputs {
    Image,
    Shader {
        time: f32,
        pointer: [f32; 4],
        textures: HashMap<String, Rc<Texture2d>>,
    },
    Blend {
        textures: HashMap<String, Rc<Texture2d>>,
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

#[derive(Debug)]
pub enum NodeOutputs {
    Texture2d(Rc<Texture2d>),
    Text(String),
    Float2([f32; 2]),
    Color([f32; 4]),
}

/// A `Node` is something that takes input and returns an output
pub trait Node {
    /// Does stuff and returns a `NodeOutputs`
    fn render(&mut self, input: &NodeInputs) -> Result<NodeOutputs, Error>;
    /// Renders to the default framebuffer
    fn present(&mut self, input: &NodeInputs) -> Result<(), Error>;
    /// Renders to a file
    fn render_to_file(&mut self, input: &NodeInputs, path: &Path) -> Result<(), Error>;
    /// Called on a window resize event
    fn resize(&mut self, width: u32, height: u32) -> Result<(), Error>;
}
