//! The text node draws text at a specified position and in a specified color

use failure::Error;
use glium::backend::Facade;
use glium::texture::{RawImage2d, Texture2d};
use glium::Surface;
use image;
use owning_ref::OwningHandle;
use std::path::Path;
use std::rc::Rc;

use opengl::{MapAsUniform, UniformsStorageVec};
use opengl::text::TextRenderer;
use super::Node;
use util::DerefInner;

/// A node that draws text
pub struct TextNode {
    /// The name of the node
    name: String,
    /// The Facade it uses to work with the OpenGL context
    facade: Rc<Facade>,
    /// The inner texture it renders to
    texture: Rc<Texture2d>,
    /// The TextRenderer it uses to render text
    text_renderer: TextRenderer,
    /// The text it draws
    text: String,
    /// The position to draw the text
    pos: [f32; 2],
    /// The color of the text in RGBA format
    color: [f32; 4],
}

impl TextNode {
    /// Create a new instance
    pub fn new(
        facade: &Rc<Facade>,
        name: String,
        text: String,
        pos: [f32; 2],
        color: [f32; 4],
        font_name: &str,
        font_size: f32,
    ) -> Result<Self, Error> {
        let (width, height) = facade.get_context().get_framebuffer_dimensions();
        let texture = Rc::new(Texture2d::empty(&**facade, width, height)?);

        let text_renderer = TextRenderer::new(facade.clone(), font_name, font_size)?;

        Ok(Self {
            name,
            facade: Rc::clone(facade),
            texture,
            text_renderer,
            text,
            pos,
            color,
        })
    }

    /// Set the text
    pub fn set_text(&mut self, text: String) {
        self.text = text;
    }

    /// Set the text position
    pub fn set_pos(&mut self, pos: [f32; 2]) {
        self.pos = pos;
    }

    /// Set the text color
    pub fn set_color(&mut self, color: [f32; 4]) {
        self.color = color;
    }

    /// Change the font by creating a new `TextRenderer`
    pub fn set_font(&mut self, font_name: &str, font_size: f32) -> Result<(), Error> {
        self.text_renderer = TextRenderer::new(self.facade.clone(), font_name, font_size)?;

        Ok(())
    }
}

impl Node for TextNode {
    fn render(&mut self, uniforms: &mut UniformsStorageVec) -> Result<(), Error> {
        let mut surface = self.texture.as_surface();

        surface.clear_color(0.0, 0.0, 0.0, 0.0);
        self.text_renderer
            .draw_text(&mut surface, &self.text, self.pos, self.color)?;

        let sampled = OwningHandle::new_with_fn(self.texture.clone(), |t| unsafe {
            DerefInner((*t).sampled())
        });
        let sampled = MapAsUniform(sampled, |s| &**s);

        uniforms.push(self.name.clone(), sampled);

        Ok(())
    }

    fn present(&mut self, _uniforms: &mut UniformsStorageVec) -> Result<(), Error> {
        let mut target = self.facade.draw();
        target.clear_color(0.0, 0.0, 0.0, 0.0);
        self.text_renderer
            .draw_text(&mut target, &self.text, self.pos, self.color)?;
        target.finish()?;

        Ok(())
    }

    fn render_to_file(
        &mut self,
        uniforms: &mut UniformsStorageVec,
        path: &Path,
    ) -> Result<(), Error> {
        self.render(uniforms)?;

        let raw: RawImage2d<u8> = self.texture.read();
        let raw = RawImage2d::from_raw_rgba_reversed(&raw.data, (raw.width, raw.height));

        image::save_buffer(path, &raw.data, raw.width, raw.height, image::RGBA(8))?;

        Ok(())
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<(), Error> {
        self.texture = Rc::new(Texture2d::empty(&*self.facade, width, height)?);

        Ok(())
    }
}
