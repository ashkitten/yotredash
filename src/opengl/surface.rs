//! Surfaces which we can render to, backed by OpenGL textures.
use glium::backend::Facade;
use glium::texture::{RawImage2d, Texture2d, Texture2dDataSource};
use std::rc::Rc;

use errors::*;
use surface::Surface;

/// A `Surface` backed by a Glium `Texture2d`.
pub struct OpenGLSurface {
    texture: Texture2d,
    facade: Rc<Facade>,
}

impl OpenGLSurface {
    /// Returns a new `OpenGLSurface`, initialized with the specified `Texture2dDataSource`.
    pub fn new<'a, T>(facade: Rc<Facade>, data: T) -> Result<OpenGLSurface>
    where
        T: Texture2dDataSource<'a>,
    {
        Ok(OpenGLSurface {
            texture: Texture2d::new(&*facade, data)?,
            facade: facade,
        })
    }

    /// Returns a reference to the inner `Texture2d`.
    pub fn ref_texture(&self) -> &Texture2d {
        &self.texture
    }
}

impl Surface for OpenGLSurface {
    fn write_buffer(&mut self, buffer: &[u8], dimensions: (u32, u32)) -> Result<()> {
        let raw = RawImage2d::from_raw_rgba_reversed(buffer, dimensions);
        self.texture = Texture2d::new(&*self.facade, raw)?;

        Ok(())
    }
}
