extern crate glium;

use glium::{glutin, Surface};

pub trait DisplayExt {
    fn init(events_loop: &glutin::EventsLoop, args: &Config) -> Self;
}

impl DisplayExt for glium::Display {
    fn init(events_loop: &glutin::EventsLoop, config: &Config) -> Self {
        let window_builder = glutin::WindowBuilder::new()
            .with_dimensions(config.buffers["__default__"].width, config.buffers["__default__"].height)
            .with_title("yotredash");

        let context = glutin::ContextBuilder::new().with_vsync(config.vsync);

        let display = glium::Display::new(window_builder, context, events_loop).unwrap();

        display
    }
}
