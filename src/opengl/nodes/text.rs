//! The text node draws text at a specified position and in a specified color

use failure::Error;
use glium::Surface;
use glium::backend::Facade;
use glium::texture::{RawImage2d, Texture2d};
use image;
use std::path::Path;
use std::rc::Rc;

use config::nodes::TextConfig;
use opengl::text::TextRenderer;
use super::{Node, NodeInputs, NodeOutputs};

/// A node that draws text
pub struct TextNode {
    /// The Facade it uses to work with the OpenGL context
    facade: Rc<Facade>,
    /// The inner texture it renders to
    texture: Rc<Texture2d>,
    /// The TextRenderer it uses to render text
    text_renderer: TextRenderer,
    /// The text it draws
    text: String,
    /// The position to draw the text
    position: [f32; 2],
    /// The color of the text in RGBA format
    color: [f32; 4],
}

impl TextNode {
    /// Create a new instance
    pub fn new(facade: &Rc<Facade>, config: TextConfig) -> Result<Self, Error> {
        let (width, height) = facade.get_context().get_framebuffer_dimensions();
        let texture = Rc::new(Texture2d::empty(&**facade, width, height)?);

        let text_renderer = TextRenderer::new(facade.clone(), &config.font_name, config.font_size)?;

        Ok(Self {
            facade: Rc::clone(facade),
            texture,
            text_renderer,
            text: config.text.or_default(),
            position: config.position.or_default(),
            color: config.color.or_default(),
        })
    }
}

impl Node for TextNode {
    fn render(&mut self, inputs: &NodeInputs) -> Result<NodeOutputs, Error> {
        if let &NodeInputs::Text {
            ref text,
            ref position,
            ref color,
        } = inputs
        {
            let text = text.clone().unwrap_or(self.text.to_string());
            let position = position.unwrap_or(self.position);
            let color = color.unwrap_or(self.color);

            let mut surface = self.texture.as_surface();
            surface.clear_color(0.0, 0.0, 0.0, 0.0);
            self.text_renderer
                .draw_text(&mut surface, &text, position.clone(), color.clone())?;

            Ok(NodeOutputs::Texture2d(Rc::clone(&self.texture)))
        } else {
            bail!("Wrong input type for node");
        }
    }

    fn present(&mut self, inputs: &NodeInputs) -> Result<(), Error> {
        if let &NodeInputs::Text {
            ref text,
            ref position,
            ref color,
        } = inputs
        {
            let text = text.clone().unwrap_or(self.text.to_string());
            let position = position.unwrap_or(self.position);
            let color = color.unwrap_or(self.color);

            let mut target = self.facade.draw();
            target.clear_color(0.0, 0.0, 0.0, 0.0);
            self.text_renderer
                .draw_text(&mut target, &text, position.clone(), color.clone())?;
            target.finish()?;
        } else {
            bail!("Wrong input type for node");
        }

        Ok(())
    }

    fn render_to_file(&mut self, inputs: &NodeInputs, path: &Path) -> Result<(), Error> {
        self.render(inputs)?;

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
