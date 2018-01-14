//! A `Source` is something that provides data to the renderer in the form of RGBA image data

// TODO: Make `Source` return a `UniformsStorage` instead of a `Frame`
// TODO: make time and pointer Sources

#[cfg(feature = "image-src")]
pub mod image;

use std::path::Path;
use failure::Error;

use surface::Surface;

#[cfg(feature = "image-src")]
pub use self::image::ImageSource;

/// A container for RGBA image data
#[derive(Clone)]
pub struct Frame {
    /// The width of the image
    pub width: u32,
    /// The height of the image
    pub height: u32,
    /// A buffer of RGBA bytes
    pub buffer: Vec<u8>,
}

impl Frame {
    /// Returns the dimensions of this frame.
    #[inline]
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

/// A `Source` is something that provides data to the renderer in the form of a `Frame` of RGBA
/// image data
pub trait Source {
    /// Create a new instance of the source, taking a `&Path` as input
    fn new(name: &str, path: &Path) -> Result<Self, Error>
    where
        Self: Sized;
    /// Get the name of the source as defined in the configuration file
    fn get_name(&self) -> &str;
    /// Does any necessary updating before rendering, returns true if changes happened
    // TODO: remove, should be unnecessary when `Source` returns a `UniformsStorage`
    fn update(&mut self) -> bool;
    /// Gets the `Frame` for rendering
    fn get_frame(&self) -> Frame;
    /// Writes the current `Frame` to a `Surface`.
    fn write_frame(&self, surface: &mut Surface) -> Result<(), Error>;
}
