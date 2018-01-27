//! A `Node` that produces values based on information about the renderer and window

use failure::Error;
use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use time::{self, Tm};

use PointerEvent;
use super::{Node, NodeInputs, NodeOutput};

/// A `Node` that produces values based on information about the renderer and window
pub struct InfoNode {
    pointer_receiver: Receiver<PointerEvent>,
    start: Tm,
    resolution: [f32; 2],
    pointer: [f32; 4],
}

impl InfoNode {
    /// Create a new instance
    pub fn new(pointer_receiver: Receiver<PointerEvent>, resolution: [f32; 2]) -> Self {
        Self {
            pointer_receiver,
            start: time::now(),
            resolution,
            pointer: [0.0; 4],
        }
    }
}

impl Node for InfoNode {
    fn render(&mut self, _inputs: &NodeInputs) -> Result<HashMap<String, NodeOutput>, Error> {
        while let Ok(pointer_event) = self.pointer_receiver.try_recv() {
            match pointer_event {
                PointerEvent::Move(x, y) => {
                    self.pointer[0] = x;
                    self.pointer[1] = self.resolution[1] - y;
                }
                PointerEvent::Press => {
                    self.pointer[2] = self.pointer[0];
                    self.pointer[3] = self.pointer[1];
                }
                PointerEvent::Release => {
                    self.pointer[2] = 0.0;
                    self.pointer[3] = 0.0;
                }
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

    fn resize(&mut self, width: u32, height: u32) -> Result<(), Error> {
        self.resolution = [width as f32, height as f32];

        Ok(())
    }
}
