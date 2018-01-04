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
//!     __default__:
//!         vertex: vertex_shader.vert
//!         fragment: fragment_shader.frag
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
extern crate notify;
extern crate owning_ref;
extern crate rect_packer;
extern crate serde_yaml;
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
pub mod errors;
pub mod font;
pub mod platform;
pub mod source;
pub mod util;

#[cfg(feature = "opengl")]
pub mod opengl;

use notify::Watcher;
use std::path::Path;
use std::sync::mpsc;
use winit::EventsLoop;

#[cfg(unix)]
use signal::Signal;
#[cfg(unix)]
use signal::trap::Trap;

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
    fn render_to_file(
        &mut self,
        time: time::Duration,
        pointer: [f32; 4],
        fps: f32,
        path: &Path,
    ) -> Result<()>;
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

fn setup_watches(
    config_path: &Path,
    config: &Config,
) -> Result<(notify::RecommendedWatcher, mpsc::Receiver<notify::RawEvent>)> {
    // Create a watcher to receive filesystem events
    let (sender, receiver) = mpsc::channel();
    let mut watcher = notify::RecommendedWatcher::new_raw(sender)?;

    // We still create the watcher, anyway, but if we're not watching anything then does it really
    // matter?
    if config.autoreload {
        // Watch the config file for changes
        watcher.watch(config_path, notify::RecursiveMode::NonRecursive)?;

        for buffer in config.buffers.values() {
            watcher.watch(
                config.path_to(&buffer.vertex),
                notify::RecursiveMode::NonRecursive,
            )?;
            watcher.watch(
                config.path_to(&buffer.fragment),
                notify::RecursiveMode::NonRecursive,
            )?;
        }

        for source in config.sources.values() {
            match source.kind.as_str() {
                "image" => watcher.watch(
                    config.path_to(&source.path),
                    notify::RecursiveMode::NonRecursive,
                )?,
                _ => (),
            }
        }
    }

    Ok((watcher, receiver))
}

quick_main!(|| -> Result<()> {
    env_logger::init()?;

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
                    WindowEvent::Resized(width, height) => {
                        actions.push(RendererAction::Resize(width, height))
                    }

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

        match receiver.try_recv() {
            Ok(notify::RawEvent {
                path,
                op: Ok(op),
                cookie: _,
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

                    actions.push(RendererAction::Reload);
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => error!("Filesystem watcher disconnected"),
            _ => (),
        }

        for action in &actions {
            match *action {
                RendererAction::Resize(width, height) => renderer.resize(width, height)?,
                RendererAction::Reload => {
                    config = Config::parse(&config_path)?;

                    // TODO: When destructuring assignment is added, change this
                    let (watcher_, receiver_) = setup_watches(&config_path, &config)?;
                    _watcher = watcher_;
                    receiver = receiver_;

                    renderer.reload(&config)?;
                }
                RendererAction::Snapshot => {
                    let path =
                        Path::new(&format!("{}.png", time::now().strftime("%F_%T")?)).to_path_buf();
                    renderer.render_to_file(time, pointer, fps_counter.fps(), &path)?
                }
                RendererAction::Close => return Ok(()),
            }
        }
    }
});
