//! The FPS counter node draws an FPS counter, using a `TextNode`

use failure::Error;
use glium::backend::Facade;
use std::path::Path;
use std::rc::Rc;

use config::nodes::{FpsConfig, NodeParameter, TextConfig};
use super::{Node, NodeInputs, NodeOutputs, TextNode};
use util::FpsCounter;

/// A node that draws text
pub struct FpsNode {
    text_node: TextNode,
    fps_counter: FpsCounter,
    position: [f32; 2],
    color: [f32; 4],
}

impl FpsNode {
    /// Create a new instance
    pub fn new(facade: &Rc<Facade>, config: FpsConfig) -> Result<Self, Error> {
        Ok(Self {
            text_node: TextNode::new(
                facade,
                TextConfig {
                    text: NodeParameter::Static("".to_string()),
                    position: config.position.clone(),
                    color: config.color.clone(),
                    font_name: config.font_name,
                    font_size: config.font_size,
                },
            )?,
            fps_counter: FpsCounter::new(config.interval),
            position: config.position.or_default(),
            color: config.color.or_default(),
        })
    }
}

impl Node for FpsNode {
    fn render(&mut self, inputs: &NodeInputs) -> Result<NodeOutputs, Error> {
        if let &NodeInputs::Fps { position, color } = inputs {
            self.fps_counter.next_frame();

            let inputs = NodeInputs::Text {
                text: Some(format!("FPS: {:.01}", self.fps_counter.fps())),
                position: Some(position.unwrap_or(self.position)),
                color: Some(color.unwrap_or(self.color)),
            };

            self.text_node.render(&inputs)
        } else {
            bail!("Wrong input type for node");
        }
    }

    fn present(&mut self, inputs: &NodeInputs) -> Result<(), Error> {
        if let &NodeInputs::Fps { position, color } = inputs {
            self.fps_counter.next_frame();

            let inputs = NodeInputs::Text {
                text: Some(format!("FPS: {:.01}", self.fps_counter.fps())),
                position: Some(position.unwrap_or(self.position)),
                color: Some(color.unwrap_or(self.color)),
            };

            self.text_node.present(&inputs)
        } else {
            bail!("Wrong input type for node");
        }
    }

    fn render_to_file(&mut self, inputs: &NodeInputs, path: &Path) -> Result<(), Error> {
        if let &NodeInputs::Fps { position, color } = inputs {
            self.fps_counter.next_frame();

            let inputs = NodeInputs::Text {
                text: Some(format!("FPS: {:.01}", self.fps_counter.fps())),
                position: Some(position.unwrap_or(self.position)),
                color: Some(color.unwrap_or(self.color)),
            };

            self.text_node.render_to_file(&inputs, path)
        } else {
            bail!("Wrong input type for node");
        }
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<(), Error> {
        self.text_node.resize(width, height)
    }
}
