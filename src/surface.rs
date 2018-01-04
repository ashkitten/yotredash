//! Generic surface abstractions for textures.
use errors::*;

/// A generic surface which we can render to.
pub trait Surface {
    /// Copies a buffer to the surface.
    fn write_buffer(&mut self, buffer: &Vec<u8>, dimensions: (u32, u32)) -> Result<()>;
}
