extern crate glium;
extern crate json;

// Glium

use glium::{glutin, Surface};

pub trait DisplayExt {
    fn init(events_loop: &glutin::EventsLoop, args: &ArgMatches) -> Self;
}

impl DisplayExt for glium::Display {
    fn init(events_loop: &glutin::EventsLoop, config: json::JsonValue) -> Self {
        let window_builder = glutin::WindowBuilder::new()
            .with_dimensions(
                config["width"].as_u32().unwrap_or(640),
                config["height"].as_u32().unwrap_or(400),
            )
            .with_title("yotredash");

        let context = glutin::ContextBuilder::new().with_vsync(config["vsync"].as_bool().unwrap_or(false));

        let display = glium::Display::new(window_builder, context, events_loop).unwrap();

        display
    }
}
