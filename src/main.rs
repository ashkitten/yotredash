//! yotredash is a an application for executing demoscene shaders

// So we don't run into issues with the error_chain macro
#![recursion_limit = "128"]
// Experimental features
#![feature(type_ascription, refcell_replace_swap, inclusive_range_syntax)]

#[cfg(unix)]
extern crate signal;

#[macro_use]
extern crate derive_error_chain;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

extern crate clap;
extern crate env_logger;
extern crate font_loader;
extern crate freetype;
extern crate nfd;
extern crate owning_ref;
extern crate rect_packer;
extern crate serde_yaml;
extern crate time;
extern crate winit;

#[cfg(feature = "opengl")]
#[macro_use]
extern crate glium;

#[cfg(feature = "image")]
extern crate image;
#[cfg(feature = "image")]
extern crate num_rational;

pub mod config;
pub mod errors;
pub mod font;
pub mod platform;
pub mod source;
pub mod util;

#[cfg(feature = "opengl")]
pub mod opengl;

#[cfg(unix)]
use signal::Signal;
#[cfg(unix)]
use signal::trap::Trap;

use std::path::Path;
use winit::EventsLoop;

#[cfg(feature = "opengl")]
use opengl::renderer::OpenGLRenderer;

use config::Config;
use errors::*;
use util::FpsCounter;

/// Renders a configured shader
pub trait Renderer {
    /// Create a new renderer
    fn new(config: Config, events_loop: &EventsLoop) -> Result<Self>
    where
        Self: Sized;
    /// Render the current frame
    fn render(&mut self, time: time::Duration, pointer: [f32; 4], fps: f32) -> Result<()>;
    /// Render the current frame to a file
    fn render_to_file(&mut self, time: time::Duration, pointer: [f32; 4], fps: f32, path: &Path) -> Result<()>;
    /// Tells the renderer to swap buffers (only applicable to buffered renderers)
    fn swap_buffers(&self) -> Result<()>;
    /// Reload the renderer from a new configuration
    fn reload(&mut self, config: &Config) -> Result<()>;
    /// Resize the renderer's output without reloading
    fn resize(&mut self, width: u32, height: u32) -> Result<()>;
}

#[derive(PartialEq)]
enum RendererAction {
    Resize(u32, u32),
    Reload,
    Snapshot,
    Close,
}

quick_main!(|| -> Result<()> {
    env_logger::init()?;

    // Register signal handler (unix only)
    #[cfg(unix)]
    let trap = Trap::trap(&[Signal::SIGUSR1, Signal::SIGUSR2, Signal::SIGHUP]);

    // Get configuration
    let config_path = Config::get_path()?.canonicalize().unwrap();
    let mut config = Config::parse(&config_path)?;

    // Creates an appropriate renderer for the configuration, exits with an error if that fails
    let mut events_loop = winit::EventsLoop::new();
    let mut renderer: Box<Renderer> = match config.renderer.as_ref() as &str {
        #[cfg(feature = "opengl")]
        "opengl" => Box::new(OpenGLRenderer::new(config.clone(), &events_loop)?),
        other => {
            error!("Renderer {} does not exist", other);
            std::process::exit(1);
        }
    };

    let mut time = time::Duration::zero();
    let mut last_frame = time::now();
    let mut fps_counter = FpsCounter::new(2.0);
    let mut pointer = [0.0; 4];

    let mut paused = false;
    loop {
        let mut actions: Vec<RendererAction> = Vec::new();

        if !paused {
            let delta = time::now() - last_frame;

            time = time + delta;
            last_frame = time::now();

            fps_counter.next_frame(delta);

            renderer.render(time, pointer, fps_counter.fps())?;
        } else {
            last_frame = time::now();

            renderer.swap_buffers()?;
        }

        #[cfg(unix)]
        {
            // Catch signals between draw calls
            let signal = trap.wait(std::time::Instant::now());
            if signal.is_some() {
                match signal.unwrap() {
                    Signal::SIGUSR1 => paused = true,
                    Signal::SIGUSR2 => paused = false,
                    Signal::SIGHUP => actions.push(RendererAction::Reload),
                    _ => (),
                }
            }
        }

        events_loop.poll_events(|event| {
            if let winit::Event::WindowEvent { event, .. } = event {
                use winit::WindowEvent;

                match event {
                    WindowEvent::Resized(width, height) => actions.push(RendererAction::Resize(width, height)),

                    WindowEvent::Closed => actions.push(RendererAction::Close),

                    WindowEvent::KeyboardInput {
                        input:
                            winit::KeyboardInput {
                                virtual_keycode: Some(keycode),
                                state: winit::ElementState::Pressed,
                                ..
                            },
                        ..
                    } => match keycode {
                        winit::VirtualKeyCode::Escape => actions.push(RendererAction::Close),
                        winit::VirtualKeyCode::F2 => actions.push(RendererAction::Snapshot),
                        winit::VirtualKeyCode::F5 => actions.push(RendererAction::Reload),
                        winit::VirtualKeyCode::F6 => paused = !paused,
                        _ => (),
                    },

                    WindowEvent::CursorMoved { position, .. } => {
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
            }
        });

        for action in &actions {
            match *action {
                RendererAction::Resize(width, height) => renderer.resize(width, height)?,
                RendererAction::Reload => {
                    config = Config::parse(&config_path)?;
                    renderer.reload(&config)?;
                }
                RendererAction::Snapshot => {
                    let path = Path::new(&format!("{}.png", time::now().strftime("%F_%T")?)).to_path_buf();
                    renderer.render_to_file(time, pointer, fps_counter.fps(), &path)?
                }
                RendererAction::Close => return Ok(()),
            }
        }
    }
});
