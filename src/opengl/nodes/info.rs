use failure::Error;
use std::sync::mpsc::Receiver;
use time::{self, Tm};

use PointerEvent;
use super::{Node, NodeInputs, NodeOutput};

pub struct InfoNode {
    pointer_receiver: Receiver<PointerEvent>,
    start: Tm,
    output_size: [f32; 2],
    pointer: [f32; 4],
}

impl InfoNode {
    pub fn new(pointer_receiver: Receiver<PointerEvent>, output_size: [f32; 2]) -> Self {
        Self {
            pointer_receiver,
            start: time::now(),
            output_size,
            pointer: [0.0; 4],
        }
    }
}

impl Node for InfoNode {
    fn render(&mut self, _inputs: &NodeInputs) -> Result<Vec<NodeOutput>, Error> {
        while let Ok(pointer_event) = self.pointer_receiver.try_recv() {
            match pointer_event {
                PointerEvent::Move(x, y) => {
                    self.pointer[0] = x;
                    self.pointer[1] = self.output_size[1] - y;
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

        Ok(vec![
            NodeOutput::Float(time),
            NodeOutput::Float2(self.output_size),
            NodeOutput::Float4(self.pointer),
        ])
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<(), Error> {
        self.output_size = [width as f32, height as f32];

        Ok(())
    }
}
