//! The text node draws text at a specified position and in a specified color

use failure::Error;
use glium::Surface;
use glium::backend::Facade;
use glium::texture::Texture2d;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc::Receiver;

use config::nodes::TextConfig;
use event::RendererEvent;
use opengl::text::TextRenderer;
use super::{Node, NodeInputs, NodeOutput};

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
    /// Receiver for events
    receiver: Receiver<RendererEvent>,
}

impl TextNode {
    /// Create a new instance
    pub fn new(
        facade: &Rc<Facade>,
        config: TextConfig,
        receiver: Receiver<RendererEvent>,
    ) -> Result<Self, Error> {
        let (width, height) = facade.get_context().get_framebuffer_dimensions();
        let texture = Rc::new(Texture2d::empty(&**facade, width, height)?);

        let text_renderer = TextRenderer::new(facade, &config.font_name, config.font_size)?;

        Ok(Self {
            facade: Rc::clone(facade),
            texture,
            text_renderer,
            text: config.text.or_default(),
            position: config.position.or_default(),
            color: config.color.or_default(),
            receiver,
        })
    }
}

impl Node for TextNode {
    fn render(&mut self, inputs: &NodeInputs) -> Result<HashMap<String, NodeOutput>, Error> {
        if let Ok(event) = self.receiver.try_recv() {
            match event {
                RendererEvent::Resize(width, height) => {
                    self.texture = Rc::new(Texture2d::empty(&*self.facade, width, height)?);
                }
                _ => (),
            }
        }

        if let NodeInputs::Text {
            ref text,
            ref position,
            ref color,
        } = *inputs
        {
            let text = text.clone().unwrap_or_else(|| self.text.to_string());
            let position = position.unwrap_or(self.position);
            let color = color.unwrap_or(self.color);

            let mut surface = self.texture.as_surface();
            surface.clear_color(0.0, 0.0, 0.0, 1.0);
            self.text_renderer
                .draw_text(&mut surface, &text, position, color)?;

            let mut outputs = HashMap::new();
            outputs.insert(
                "texture".to_string(),
                NodeOutput::Texture2d(Rc::clone(&self.texture)),
            );
            Ok(outputs)
        } else {
            bail!("Wrong input type for node");
        }
    }
}
