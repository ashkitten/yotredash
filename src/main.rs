//! An application for executing demoscene shaders.
//!
//! Yotredash is entirely separate from [Shadertoy](https://shadertoy.com), and does not intend to
//! be directly compatible with shaders created for Shadertoy. However, it does intend to reach at
//! least feature parity with Shadertoy, so that shaders might be easily ported to Yotredash.
//!
//! # Configuration
//! Yotredash provides a simple yaml configuration from which a user can configure nearly all
//! behaviors of the application.
//!
//! ```yaml
//! buffers:
//!     output:
//!         type: output
//!         texture:
//!             node: shader
//!             output: texture
//!
//!     shader:
//!         type: shader
//!         vertex: vertex_shader.vert
//!         fragment: fragment_shader.frag
//!         uniforms:
//!             -
//!                 node: info
//!                 output: resolution
//!
//!     info:
//!         type: info
//! ```
//!
//! It also provides command line options which can be used to quickly override options in the
//! configuration.
//!
//! ```shell
//! yotredash --config path/to/config.yml --fullscreen
//! ```
//!
//! The above example will run yotredash in fullscreen mode, regardless of whether or not the
//! `fullscreen` option is specified in the configuration file.

// Warn if things are missing documentation
#![warn(missing_docs)]

#[cfg(unix)]
extern crate signal;

#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

extern crate clap;
extern crate env_logger;
extern crate font_loader;
extern crate freetype;
extern crate nfd;
extern crate notify;
extern crate owning_ref;
extern crate rect_packer;
extern crate serde_yaml;
extern crate solvent;
extern crate time;
extern crate winit;

#[cfg(feature = "opengl")]
#[macro_use]
extern crate glium;

#[cfg(feature = "image-src")]
extern crate gif;
#[cfg(feature = "image-src")]
extern crate gif_dispose;
#[cfg(feature = "image-src")]
extern crate image;

pub mod config;
pub mod event;
pub mod font;
pub mod platform;
pub mod util;

#[cfg(feature = "opengl")]
pub mod opengl;

use notify::Watcher;
use std::path::Path;
use std::sync::mpsc;
use failure::Error;

#[cfg(unix)]
use signal::Signal;
#[cfg(unix)]
use signal::trap::Trap;

#[cfg(feature = "opengl")]
use opengl::renderer::OpenGLRenderer;

use config::Config;
use config::nodes::NodeConfig;
use event::*;

/// Renders a configured shader
pub trait Renderer {
    /// Render the current frame
    fn render(&mut self) -> Result<(), Error>;
    /// Tells the renderer to swap buffers (only applicable to buffered renderers)
    fn swap_buffers(&self) -> Result<(), Error>;
}

fn setup_watches(
    config_path: &Path,
    config: &Config,
) -> Result<(notify::RecommendedWatcher, mpsc::Receiver<notify::RawEvent>), Error> {
    // Create a watcher to receive filesystem events
    let (sender, receiver) = mpsc::channel();
    let mut watcher = notify::RecommendedWatcher::new_raw(sender)?;

    // We still create the watcher, anyway, but if we're not watching anything then does it really
    // matter?
    if config.autoreload {
        // Watch the config file for changes
        watcher.watch(config_path, notify::RecursiveMode::NonRecursive)?;

        for node in config.nodes.values() {
            match *node {
                NodeConfig::Image(ref image_config) => watcher.watch(
                    config.path_to(Path::new(&image_config.path)),
                    notify::RecursiveMode::NonRecursive,
                )?,
                NodeConfig::Shader(ref shader_config) => {
                    watcher.watch(
                        config.path_to(Path::new(&shader_config.vertex)),
                        notify::RecursiveMode::NonRecursive,
                    )?;
                    watcher.watch(
                        config.path_to(Path::new(&shader_config.fragment)),
                        notify::RecursiveMode::NonRecursive,
                    )?;
                }
                _ => (),
            }
        }
    }

    Ok((watcher, receiver))
}

fn run() -> Result<(), Error> {
    env_logger::try_init()?;

    // Register signal handler (unix only)
    #[cfg(unix)]
    let trap = Trap::trap(&[Signal::SIGUSR1, Signal::SIGUSR2, Signal::SIGHUP]);

    // Get configuration
    let config_path = Config::get_path()?;
    let mut config = Config::parse(&config_path)?;

    // Setup filesystem watches
    let (mut _watcher, mut receiver) = setup_watches(&config_path, &config)?;

    // Creates an appropriate renderer for the configuration, exits with an error if that fails
    let mut events_loop = winit::EventsLoop::new();
    let (event_sender, event_receiver) = mpsc::channel();
    let mut renderer: Box<Renderer> = match config.renderer.as_ref() as &str {
        #[cfg(feature = "opengl")]
        "opengl" => Box::new(OpenGLRenderer::new(
            config.clone(),
            &events_loop,
            event_receiver,
        )?),
        other => {
            error!("Renderer {} does not exist", other);
            std::process::exit(1);
        }
    };

    let mut paused = false;
    loop {
        let mut events: Vec<Event> = Vec::new();

        if !paused {
            renderer.render()?;
        } else {
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
                    Signal::SIGHUP => events.push(Event::Reload),
                    _ => (),
                }
            }
        }

        events_loop.poll_events(|event| {
            if let winit::Event::WindowEvent { event, .. } = event {
                use winit::WindowEvent;

                match event {
                    WindowEvent::Resized(width, height) => {
                        events.push(Event::Resize(width, height))
                    }

                    WindowEvent::Closed => events.push(Event::Close),

                    WindowEvent::KeyboardInput {
                        input:
                            winit::KeyboardInput {
                                virtual_keycode: Some(keycode),
                                state: winit::ElementState::Pressed,
                                ..
                            },
                        ..
                    } => match keycode {
                        winit::VirtualKeyCode::Escape => events.push(Event::Close),
                        winit::VirtualKeyCode::F2 => events.push(Event::Capture),
                        winit::VirtualKeyCode::F5 => events.push(Event::Reload),
                        winit::VirtualKeyCode::F6 => paused = !paused,
                        _ => (),
                    },

                    WindowEvent::CursorMoved { position, .. } => {
                        events.push(Event::Pointer(PointerEvent::Move(
                            position.0 as f32,
                            position.1 as f32,
                        )));
                    }

                    WindowEvent::MouseInput {
                        button: winit::MouseButton::Left,
                        state,
                        ..
                    } => match state {
                        winit::ElementState::Pressed => {
                            events.push(Event::Pointer(PointerEvent::Press));
                        }
                        winit::ElementState::Released => {
                            events.push(Event::Pointer(PointerEvent::Release));
                        }
                    },

                    _ => (),
                }
            }
        });

        match receiver.try_recv() {
            Ok(notify::RawEvent {
                path, op: Ok(op), ..
            }) => {
                // We listen for both WRITE and REMOVE events because some editors (like vim) will
                // remove the file and write a new one in its place, and on Linux this will also
                // remove the watch, so we won't ever receive a WRITE event in this case
                if op.intersects(notify::op::WRITE | notify::op::REMOVE) {
                    if let Some(path) = path {
                        info!(
                            "Detected file change for {}, reloading...",
                            path.to_str().unwrap()
                        );
                    } else {
                        info!("Detected file change, reloading...");
                    }

                    events.push(Event::Reload);
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => error!("Filesystem watcher disconnected"),
            _ => (),
        }

        for event in events {
            match event {
                Event::Pointer(pointer_event) => {
                    event_sender.send(RendererEvent::Pointer(pointer_event))?;
                }
                Event::Resize(width, height) => {
                    event_sender.send(RendererEvent::Resize(width, height))?;
                }
                Event::Reload => {
                    config = Config::parse(&config_path)?;

                    // TODO: When destructuring assignment is added, change this
                    let (watcher_, receiver_) = setup_watches(&config_path, &config)?;
                    _watcher = watcher_;
                    receiver = receiver_;

                    event_sender.send(RendererEvent::Reload(config))?;
                }
                Event::Capture => {
                    let path =
                        Path::new(&format!("{}.png", time::now().strftime("%F_%T")?)).to_path_buf();
                    event_sender.send(RendererEvent::Capture(path))?;
                }
                Event::Close => return Ok(()),
            }
        }
    }
}

fn main() {
    use std::io::Write;

    std::process::exit(match run() {
        Ok(()) => 0,
        Err(ref error) => {
            let mut causes = error.causes();

            error!(
                "{}",
                causes
                    .next()
                    .expect("`causes` should contain at least one error")
            );
            for cause in causes {
                error!("Caused by: {}", cause);
            }

            let backtrace = format!("{}", error.backtrace());
            if backtrace.is_empty() {
                writeln!(
                    ::std::io::stderr(),
                    "Set RUST_BACKTRACE=1 to see a backtrace"
                ).expect("Could not write to stderr");
            } else {
                writeln!(::std::io::stderr(), "{}", error.backtrace())
                    .expect("Could not write to stderr");
            }

            1
        }
    });
}
