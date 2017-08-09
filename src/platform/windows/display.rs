extern crate glium;

use glium::{glutin, Surface};
use Args;

pub trait DisplayExt {
    fn init(events_loop: &glutin::EventsLoop, args: &Args) -> Self;
}

impl DisplayExt for glium::Display {
    fn init(events_loop: &glutin::EventsLoop, args: &Args) -> Self {
        let window_builder = glutin::WindowBuilder::new()
            .with_dimensions(args.width, args.height);

        let context = glutin::ContextBuilder::new()
            .with_vsync(args.vsync);

        let display = glium::Display::new(window_builder, context, &events_loop).unwrap();

        return display;
    }
}
