#[cfg(feature = "image-src")]
pub mod image;

use std::path::Path;

use errors::*;

#[cfg(feature = "image-src")]
pub use self::image::ImageSource;

#[derive(Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub buffer: Vec<u8>,
}

pub trait Source {
    fn new(path: &Path) -> Result<Self>
    where
        Self: Sized;
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn update(&mut self) -> bool;
    fn get_frame(&self) -> Frame;
}
