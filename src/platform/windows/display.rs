extern crate glium;

// Glium

use glium::{glutin, Surface};

// Clap

use clap::ArgMatches;

pub trait DisplayExt {
    fn init(events_loop: &glutin::EventsLoop, args: &Args) -> Self;
}

impl DisplayExt for glium::Display {
    fn init(events_loop: &glutin::EventsLoop, args: &Args) -> Self {
        let window_builder = glutin::WindowBuilder::new()
            .with_dimensions(
                args.value_of("width")
                    .unwrap_or("640")
                    .parse::<u32>()
                    .unwrap(),
                args.value_of("height")
                    .unwrap_or("400")
                    .parse::<u32>()
                    .unwrap(),
            )
            .with_title("yotredash");

        let context = glutin::ContextBuilder::new().with_vsync(args.is_present("vsync"));

        let display = glium::Display::new(window_builder, context, events_loop).unwrap();

        display
    }
}
