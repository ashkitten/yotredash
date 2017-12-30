use config::Config;
use winit::EventsLoop;

pub trait Renderer {
    fn new(config: &Config, events_loop: &EventsLoop) -> Self
    where
        Self: Sized;
    fn render(&mut self, pointer: [f32; 4]);
    fn swap_buffers(&self);
    fn reload(&mut self, config: &Config);
    fn resize(&mut self, width: u32, height: u32);
}
