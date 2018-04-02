//! A `Node` that takes an initial input and a node connection and loops it back into a node to
//! avoid dependency cycles

use failure::Error;
use glium::backend::Facade;
use glium::texture::{Texture1d, Texture2d};
use std::collections::HashMap;
use std::rc::Rc;

use config::nodes::{FeedbackConfig, InputType, NodeConnection};
use super::{Node, NodeInputs, NodeOutput};

/// A `Node` that reads an image from file and returns frames from that image
pub struct FeedbackNode {
    values: HashMap<String, NodeOutput>,
}

impl FeedbackNode {
    /// Create a new instance
    pub fn new(facade: &Rc<Facade>, config: FeedbackConfig) -> Result<Self, Error> {
        let mut values = HashMap::new();

        for input in config.inputs {
            match input.type_ {
                InputType::Any => bail!("Must specify `type` for inputs of feedback node"),
                InputType::Color => {
                    values.insert(input.name, NodeOutput::Color(Default::default()))
                }
                InputType::Float => {
                    values.insert(input.name, NodeOutput::Float(Default::default()))
                }
                InputType::Float2 => {
                    values.insert(input.name, NodeOutput::Float2(Default::default()))
                }
                InputType::Float4 => {
                    values.insert(input.name, NodeOutput::Float4(Default::default()))
                }
                InputType::Text => values.insert(input.name, NodeOutput::Text(Default::default())),
                InputType::Texture2d => values.insert(
                    input.name,
                    NodeOutput::Texture2d(Rc::new(Texture2d::empty(&**facade, 0, 0)?)),
                ),
                InputType::Texture1d => values.insert(
                    input.name,
                    NodeOutput::Texture1d(Rc::new(Texture1d::empty(&**facade, 0)?)),
                ),
            };
        }

        Ok(Self { values })
    }

    /// Update values
    pub fn update(&mut self, inputs: &HashMap<NodeConnection, NodeOutput>) {
        for (connection, output) in inputs {
            self.values
                .insert(connection.name.to_string(), output.clone());
        }
    }
}

impl Node for FeedbackNode {
    fn render(&mut self, _inputs: &NodeInputs) -> Result<HashMap<String, NodeOutput>, Error> {
        Ok(self.values.clone())
    }
}
