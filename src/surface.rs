use std::rc::Rc;
use std::cell::RefCell;
use glium::backend::Facade;
use glium::texture::{RawImage2d, Texture2d, Texture2dDataSource};

use errors::*;

/// A generic surface which we can render to.
pub trait Surface {
	/// Copies a buffer to the surface.
    fn write_buffer(&mut self, buffer: &Vec<u8>, dimensions: (u32, u32)) -> Result<()>;
}

/// A `Surface` backed by a Glium `Texture2d`.
pub struct OpenGLSurface {
	pub texture: Texture2d,
	facade: Rc<Facade>,
}

impl OpenGLSurface {
	pub fn new<'a, T>(facade: Rc<Facade>, data: T) -> Result<OpenGLSurface>
		where T: Texture2dDataSource<'a>
	{
		Ok(OpenGLSurface {
			texture: Texture2d::new(&*facade, data)?,
			facade: facade,
		})
	}
}

impl Surface for OpenGLSurface {
	fn write_buffer(&mut self, buffer: &Vec<u8>, dimensions: (u32, u32)) -> Result<()> {
		let raw = RawImage2d::from_raw_rgba_reversed(buffer, dimensions);
		self.texture = Texture2d::new(&*self.facade, raw)?;

		Ok(())
	}
}