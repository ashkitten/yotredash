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

mod config;
mod platform;
mod renderer;

#[cfg(feature = "opengl")]
mod opengl;

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

    let mut start_time = time::now();
    let mut pointer = [0.0; 4];

    let mut closed = false;
    let mut paused = false;
    let mut last_frame = time::now();
    let mut frames = 0.0;
    while !closed {
        if !paused {
            let time =
                (((time::now() - start_time).num_nanoseconds().unwrap() as f64) / 1000_000_000.0 % 4096.0) as f32;
            renderer.render(time, pointer);
        } else {
            // TODO: swap buffers when paused
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
                        info!("Restarting!");
                        config = Config::parse();
                        renderer.reload(&config);
                        start_time = time::now();
                    }
                    _ => (),
                }
            }
        }

        if config.fps {
            let delta = time::now() - last_frame;
            frames += 1.0;
            if delta > time::Duration::seconds(5) {
                println!("FPS: {}", frames / (delta.num_nanoseconds().unwrap() as f64 / 1_000_000_000.0));
                last_frame = time::now();
                frames = 0.0;
            }
        }

        events_loop.poll_events(|event| if let winit::Event::WindowEvent { event, .. } = event {
            use winit::WindowEvent;

            match event {
                WindowEvent::Resized(width, height) => {
                    renderer.resize(width, height);
                }
                WindowEvent::Closed |
                WindowEvent::KeyboardInput {
                    input: winit::KeyboardInput {
                        virtual_keycode: Some(winit::VirtualKeyCode::Escape),
                        ..
                    },
                    ..
                } => closed = true,
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
