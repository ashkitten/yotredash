#[cfg(feature = "image-src")]
pub mod image;
pub mod buffer;

use failure::Error;
use std::path::Path;

#[cfg(feature = "image-src")]
pub use self::image::ImageNode;

pub use self::buffer::BufferNode;
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
