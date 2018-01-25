//! The FPS counter node draws an FPS counter, using a `TextNode`

use failure::Error;
use glium::backend::Facade;
use std::path::Path;
use std::rc::Rc;

use opengl::UniformsStorageVec;
use super::{Node, TextNode};
use util::FpsCounter;

/// A node that draws text
pub struct FpsNode {
    text_node: TextNode,
    fps_counter: FpsCounter,
}

impl FpsNode {
    /// Create a new instance
    pub fn new(
        facade: &Rc<Facade>,
        name: String,
        position: [f32; 2],
        color: [f32; 4],
        font_name: &str,
        font_size: f32,
        interval: f32,
    ) -> Result<Self, Error> {
        Ok(Self {
            text_node: TextNode::new(facade, name, String::default(), position, color, font_name, font_size)?,
            fps_counter: FpsCounter::new(interval),
        })
    }

    /// Set the text position
    pub fn set_position(&mut self, position: [f32; 2]) {
        self.text_node.set_position(position);
    }

    /// Set the text color
    pub fn set_color(&mut self, color: [f32; 4]) {
        self.text_node.set_color(color);
    }

    /// Change the font by creating a new `TextRenderer`
    pub fn set_font(&mut self, font_name: &str, font_size: f32) -> Result<(), Error> {
        self.text_node.set_font(font_name, font_size)
    }
}

impl Node for FpsNode {
    fn render(&mut self, uniforms: &mut UniformsStorageVec) -> Result<(), Error> {
        self.fps_counter.next_frame();
        self.text_node.set_text(format!("FPS: {:.01}", self.fps_counter.fps()));
        self.text_node.render(uniforms)
    }

    fn present(&mut self, uniforms: &mut UniformsStorageVec) -> Result<(), Error> {
        self.fps_counter.next_frame();
        self.text_node.set_text(format!("FPS: {:.01}", self.fps_counter.fps()));
        self.text_node.present(uniforms)
    }

    fn render_to_file(
        &mut self,
        uniforms: &mut UniformsStorageVec,
        path: &Path,
    ) -> Result<(), Error> {
        self.fps_counter.next_frame();
        self.text_node.set_text(format!("FPS: {:.01}", self.fps_counter.fps()));
        self.text_node.render_to_file(uniforms, path)
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<(), Error> {
        self.text_node.resize(width, height)
    }
}
