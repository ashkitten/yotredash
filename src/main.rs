#[cfg(unix)]
extern crate signal;

#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

extern crate clap;
extern crate env_logger;
extern crate image;
extern crate owning_ref;
extern crate time;
extern crate winit;

#[cfg(feature = "opengl")]
#[macro_use]
extern crate glium;

#[cfg(feature = "font-rendering")]
extern crate freetype;

mod config;
mod platform;
mod renderer;

#[cfg(feature = "opengl")]
mod opengl;

#[cfg(feature = "font-rendering")]
mod font;

#[cfg(unix)]
use signal::Signal;
#[cfg(unix)]
use signal::trap::Trap;

#[cfg(feature = "opengl")]
use opengl::renderer::OpenGLRenderer;

use config::Config;
use renderer::Renderer;

fn main() {
    env_logger::init().unwrap();

    // Register signal handler
    #[cfg(unix)]
    let trap = Trap::trap(&[Signal::SIGUSR1, Signal::SIGUSR2, Signal::SIGHUP]);

    let mut config = Config::parse();

    let mut events_loop = winit::EventsLoop::new();
    let mut renderer: Box<Renderer> = match config.renderer.as_ref() as &str {
        #[cfg(feature = "opengl")]
        "opengl" => Box::new(OpenGLRenderer::new(&config, &events_loop)),
        other => {
            error!("Renderer {} does not exist", other);
            std::process::exit(1);
        }
    };

    let mut pointer = [0.0; 4];

    let mut closed = false;
    let mut paused = false;
    while !closed {
        if !paused {
            renderer.render(pointer);
        } else {
            renderer.swap_buffers();
        }

        #[cfg(unix)]
        {
            // Catch signals between draw calls
            let signal = trap.wait(std::time::Instant::now());
            if signal.is_some() {
                match signal.unwrap() {
                    Signal::SIGUSR1 => paused = true,
                    Signal::SIGUSR2 => paused = false,
                    Signal::SIGHUP => {
                        config = Config::parse();
                        renderer.reload(&config);
                    }
                    _ => (),
                }
            }
        }

        events_loop.poll_events(|event| if let winit::Event::WindowEvent { event, .. } = event {
            use winit::WindowEvent;

            match event {
                WindowEvent::Resized(width, height) => renderer.resize(width, height),

                WindowEvent::Closed => closed = true,

                WindowEvent::KeyboardInput {
                    input: winit::KeyboardInput {
                        virtual_keycode: Some(keycode),
                        state: winit::ElementState::Pressed,
                        ..
                    },
                    ..
                } => match keycode {
                    winit::VirtualKeyCode::Escape => closed = true,
                    winit::VirtualKeyCode::F5 => {
                        config = Config::parse();
                        renderer.reload(&config);
                    }
                    _ => (),
                },

                WindowEvent::MouseMoved { position, .. } => {
                    pointer[0] = position.0 as f32;
                    pointer[1] = position.1 as f32;
                }

                WindowEvent::MouseInput {
                    button: winit::MouseButton::Left,
                    state,
                    ..
                } => match state {
                    winit::ElementState::Pressed => {
                        pointer[2] = pointer[0];
                        pointer[3] = pointer[1];
                    }
                    winit::ElementState::Released => {
                        pointer[2] = 0.0;
                        pointer[3] = 0.0;
                    }
                },

                _ => (),
            }
        });
    }
}
