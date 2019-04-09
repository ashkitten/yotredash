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
#![feature(c_variadic)]

use env_logger;
use failure::{format_err, Error};
use log::{error, info, warn};
use notify::{self, Watcher};
use std::{path::Path, sync::mpsc};
use time;
use winit;

pub mod config;
pub mod clog;
pub mod event;
pub mod font;
pub mod opengl;
pub mod platform;
pub mod renderer;
pub mod util;

#[cfg(unix)]
use signal::trap::Trap;
#[cfg(unix)]
use signal::Signal;

use crate::{
    config::{nodes::NodeConfig, Config},
    event::*,
    opengl::renderer::{OpenGLDebugRenderer, OpenGLRenderer},
    renderer::{DebugRenderer, Renderer},
    util::format_error,
};

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

fn main() -> Result<(), Error> {
    crate::clog::setup_c_logging();

    env_logger::Builder::from_default_env()
        .format(|buf, record| {
            use env_logger::fmt::Color;
            use log::Level;
            use std::io::Write;

            let level = record.level();
            let mut level_style = buf.style();
            match level {
                Level::Trace => level_style.set_color(Color::White),
                Level::Debug => level_style.set_color(Color::Blue),
                Level::Info => level_style.set_color(Color::Green),
                Level::Warn => level_style.set_color(Color::Yellow),
                Level::Error => level_style.set_color(Color::Red).set_bold(true),
            };
            writeln!(
                buf,
                "{:>5} {}: {}",
                level_style.value(level),
                record.target(),
                record.args()
            )
        })
        .init();

    // For catching and displaying errors
    let mut error = None;

    // Register signal handler (unix only)
    #[cfg(unix)]
    let trap = Trap::trap(&[Signal::SIGUSR1, Signal::SIGUSR2, Signal::SIGHUP]);

    // Get configuration
    let config_path = Config::get_path()?;
    let config = match Config::parse(&config_path) {
        Ok(config) => config,
        Err(e) => {
            error!("{}", format_error(&e));
            error = Some(e);
            Config::backup()?
        }
    };

    // Setup filesystem watches
    let (mut watcher, mut receiver) = setup_watches(&config_path, &config)?;

    // Creates an appropriate renderer for the configuration, exits with an error if that fails
    let mut events_loop = winit::EventsLoop::new();

    let (mut event_sender, event_receiver) = mpsc::channel();
    // TODO: return something renderer-independent instead of Facade
    let (mut renderer, mut debug_renderer, facade) = match config.renderer.as_ref() as &str {
        "opengl" => {
            let facade = opengl::renderer::new_facade(&config, &events_loop)?;
            let renderer = match OpenGLRenderer::new(&config, &facade, event_receiver) {
                Ok(r) => Some(Box::new(r)),
                Err(e) => {
                    error = Some(e);
                    None
                }
            };
            let debug_renderer = OpenGLDebugRenderer::new(&facade)?;
            (renderer, Box::new(debug_renderer), facade)
        }
        other => {
            let facade = opengl::renderer::new_facade(&config, &events_loop)?;
            let debug_renderer = OpenGLDebugRenderer::new(&facade)?;
            error = Some(format_err!("Renderer {} is not built in", other));
            (None, Box::new(debug_renderer), facade)
        }
    };

    let mut paused = false;
    loop {
        let mut events: Vec<Event> = Vec::new();

        if let Some(ref mut renderer) = renderer {
            renderer.update()?;
        }

        match error {
            None => {
                if let Some(ref mut renderer) = renderer {
                    if !paused {
                        match renderer.render() {
                            Err(e) => {
                                error!("{}", format_error(&e));
                                error = Some(e);
                            }
                            _ => (),
                        }
                    } else {
                        match renderer.swap_buffers() {
                            Err(e) => {
                                error!("{}", format_error(&e));
                                error = Some(e);
                            }
                            _ => (),
                        }
                    }
                }
            }
            Some(ref error) => debug_renderer.draw_error(error)?,
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
                    WindowEvent::Resized(size) => {
                        let size: (u32, u32) = size.into();
                        events.push(Event::Resize(size.0, size.1))
                    }

                    WindowEvent::CloseRequested => events.push(Event::Close),

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
                            position.x as f32,
                            position.y as f32,
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
                    if let Some(ref path) = path {
                        info!(
                            "Detected file change for {}, reloading...",
                            path.to_str().unwrap()
                        );
                    } else {
                        info!("Detected file change, reloading...");
                    }

                    events.push(Event::Reload);
                }

                // If the file was removed and replaced (how certain editors save files)
                if op.contains(notify::op::REMOVE) {
                    if let Some(path) = path {
                        if path.exists() {
                            watcher.watch(path, notify::RecursiveMode::NonRecursive)?;
                        }
                    }
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => error!("Filesystem watcher disconnected"),
            _ => (),
        }

        for event in events {
            match event {
                Event::Pointer(pointer_event) => {
                    if renderer.is_some() {
                        event_sender.send(RendererEvent::Pointer(pointer_event))?;
                    }
                }
                Event::Resize(..) => {
                    if renderer.is_some() {
                        let (width, height) = facade.get_context().get_framebuffer_dimensions();
                        event_sender.send(RendererEvent::Resize(width, height))?;
                    }
                }
                Event::Reload => {
                    match Config::parse(&config_path) {
                        Ok(config) => {
                            // TODO: When destructuring assignment is added, change this
                            let (watcher_, receiver_) = setup_watches(&config_path, &config)?;
                            watcher = watcher_;
                            receiver = receiver_;

                            let (event_sender_, event_receiver) = mpsc::channel();
                            event_sender = event_sender_;

                            renderer = match config.renderer.as_ref() as &str {
                                "opengl" => {
                                    match OpenGLRenderer::new(&config, &facade, event_receiver) {
                                        Ok(r) => {
                                            error = None;
                                            Some(Box::new(r))
                                        }
                                        Err(e) => {
                                            error = Some(e);
                                            None
                                        }
                                    }
                                }
                                other => {
                                    error = Some(format_err!("Renderer {} is not built in", other));
                                    None
                                }
                            }
                        }
                        Err(e) => {
                            error!("{}", format_error(&e));
                            error = Some(e);
                        }
                    }
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
