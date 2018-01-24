//! Nodes are the basic building blocks for the renderer.
// TODO: expand documentation and add examples

#[cfg(feature = "image-src")]
pub mod image;

pub mod blend;
pub mod shader;

use failure::Error;
use std::path::Path;

#[cfg(feature = "image-src")]
pub use self::image::ImageNode;

pub use self::blend::BlendNode;
pub use self::shader::ShaderNode;
use super::UniformsStorageVec;

/// A `Node` is something that takes input as a UniformsStorage and returns data in a
/// UniformsStorage
pub trait Node {
    /// Does stuff and puts its value in the UniformsStorageVec
    fn render(&mut self, input: &mut UniformsStorageVec) -> Result<(), Error>;
    /// Renders to the default framebuffer
    fn present(&mut self, input: &mut UniformsStorageVec) -> Result<(), Error>;
    /// Renders to a file
    fn render_to_file(&mut self, input: &mut UniformsStorageVec, path: &Path) -> Result<(), Error>;
}