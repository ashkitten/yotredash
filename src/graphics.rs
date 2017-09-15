use glium::texture::PixelValue;

use errors::*;
use font::RenderedGlyph;

pub struct Texture<P> where P: Clone {
    pub data: Vec<P>,
    pub width: u32,
    pub height: u32,
}

pub trait GpuTexture<B> where B: ?Sized {
    fn new<P>(backend: &B, texture: Texture<P>) -> Result<Self> where Self: Sized, P: Clone + PixelValue;
}

pub trait GpuGlyph<B> where B: ?Sized {
    fn new(backend: &B, glyph: RenderedGlyph) -> Result<Self> where Self: Sized;
}
