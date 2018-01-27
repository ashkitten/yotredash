//! A `Node` that produces values based on information about the renderer and window

use failure::Error;
use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use time::{self, Tm};

use event::{PointerEvent, RendererEvent};
use super::{Node, NodeInputs, NodeOutput};

/// A `Node` that produces values based on information about the renderer and window
pub struct InfoNode {
    receiver: Receiver<RendererEvent>,
    start: Tm,
    resolution: [f32; 2],
    pointer: [f32; 4],
}

impl InfoNode {
    /// Create a new instance
    pub fn new(receiver: Receiver<RendererEvent>, resolution: [f32; 2]) -> Self {
        Self {
            receiver,
            start: time::now(),
            resolution,
            pointer: [0.0; 4],
        }
    }
}

impl Node for InfoNode {
    fn render(&mut self, _inputs: &NodeInputs) -> Result<HashMap<String, NodeOutput>, Error> {
        while let Ok(event) = self.receiver.try_recv() {
            match event {
                RendererEvent::Pointer(PointerEvent::Move(x, y)) => {
                    self.pointer[0] = x;
                    self.pointer[1] = self.resolution[1] - y;
                }
                RendererEvent::Pointer(PointerEvent::Press) => {
                    self.pointer[2] = self.pointer[0];
                    self.pointer[3] = self.pointer[1];
                }
                RendererEvent::Pointer(PointerEvent::Release) => {
                    self.pointer[2] = 0.0;
                    self.pointer[3] = 0.0;
                }
                RendererEvent::Resize(width, height) => {
                    self.resolution = [width as f32, height as f32];
                }
                _ => (),
            }
        }

        let time = ((time::now() - self.start).num_nanoseconds().unwrap() as f32) / 1000_000_000.0
            % 4096.0;

        let mut outputs = HashMap::new();
        outputs.insert("time".to_string(), NodeOutput::Float(time));
        outputs.insert(
            "resolution".to_string(),
            NodeOutput::Float2(self.resolution),
        );
        outputs.insert("pointer".to_string(), NodeOutput::Float4(self.pointer));
        Ok(outputs)
    }
}
