//! An implementation of `Renderer` using OpenGL

use failure::Error;
use failure::SyncFailure;
use glium::backend::Facade;
use glium::backend::glutin::Display;
use glium::backend::glutin::headless::Headless;
use glium::glutin::{ContextBuilder, GlProfile, HeadlessRendererBuilder, WindowBuilder};
use solvent::DepGraph;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use winit::EventsLoop;

use Renderer;
use config::{Config, NodeConfig};
use super::UniformsStorageVec;
use super::nodes::{BufferNode, ImageNode, Node};

/// An implementation of a `Renderer` which uses OpenGL
pub struct OpenGLRenderer {
    /// The facade it uses to render
    facade: Rc<Facade>,
    /// Maps names to nodes
    nodes: HashMap<String, Box<Node>>,
    /// Order to render nodes in
    order: Vec<String>,
}

impl Renderer for OpenGLRenderer {
    fn new(config: Config, events_loop: &EventsLoop) -> Result<Self, Error> {
        let facade: Rc<Facade> = if !config.headless {
            let window_builder = WindowBuilder::new()
                .with_dimensions(config.width, config.height)
                .with_title("yotredash")
                .with_maximized(config.maximize)
                .with_fullscreen(if config.fullscreen {
                    Some(events_loop.get_primary_monitor())
                } else {
                    None
                });
            let context_builder = ContextBuilder::new()
                .with_vsync(config.vsync)
                .with_srgb(false);
            let display = Display::new(window_builder, context_builder, events_loop)
                .map_err(SyncFailure::new)?;
            ::platform::window::init(display.gl_window().window(), &config);

            Rc::new(display)
        } else {
            let context = HeadlessRendererBuilder::new(config.width, config.height)
                .with_gl_profile(GlProfile::Core)
                .build()
                .map_err(SyncFailure::new)?;
            Rc::new(Headless::new(context)?)
        };

        debug!(
            "OpenGL backend: {}",
            facade.get_context().get_opengl_version_string()
        );

        let mut nodes: HashMap<String, Box<Node>> = HashMap::new();
        let mut dep_graph: DepGraph<&str> = DepGraph::new();
        for (name, node_config) in config.nodes.iter() {
            match *node_config {
                NodeConfig::Image { ref path } => {
                    nodes.insert(
                        name.to_string(),
                        Box::new(ImageNode::new(&facade, name.to_string(), &config.path_to(path))?),
                    );
                }
                NodeConfig::Buffer {
                    ref vertex,
                    ref fragment,
                    ref dependencies,
                } => {
                    nodes.insert(
                        name.to_string(),
                        Box::new(BufferNode::new(
                            &facade,
                            name.to_string(),
                            &config.path_to(vertex),
                            &config.path_to(fragment),
                        )?),
                    );

                    dep_graph.register_dependencies(name, dependencies.iter().map(|dep| dep.as_str()).collect());
                }
            }
        }

        let mut order = Vec::new();
        for dep in dep_graph.dependencies_of(&"__default__")? {
            order.push(dep?.to_string());
        }

        Ok(Self {
            facade,
            nodes,
            order,
        })
    }

    fn render(&mut self, time: ::time::Duration, pointer: [f32; 4], fps: f32) -> Result<(), Error> {
        let time = (time.num_nanoseconds().unwrap() as f32) / 1000_000_000.0 % 4096.0;
        let pointer = {
            let height = self.facade.get_context().get_framebuffer_dimensions().1 as f32;
            [
                pointer[0],
                height - pointer[1],
                pointer[2],
                height - pointer[3],
            ]
        };

        let mut uniforms = UniformsStorageVec::new();
        uniforms.push("time".to_string(), time);
        uniforms.push("pointer".to_string(), pointer);

        for name in &self.order {
            if name == "__default__" {
                self.nodes
                    .get_mut(name)
                    .unwrap()
                    .present(&mut uniforms)?;
            } else {
                let output = self.nodes
                    .get_mut(name)
                    .unwrap()
                    .render(&mut uniforms)?;
            }
        }

        Ok(())
    }

    fn swap_buffers(&self) -> Result<(), Error> {
        self.facade.get_context().swap_buffers()?;
        Ok(())
    }

    fn reload(&mut self, config: &Config) -> Result<(), Error> {
        info!("Reloading config");
        // TODO: reimplement
        Ok(())
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<(), Error> {
        debug!("Resized window to {}x{}", width, height);
        // TODO: reimplement
        Ok(())
    }

    fn render_to_file(
        &mut self,
        time: ::time::Duration,
        pointer: [f32; 4],
        fps: f32,
        path: &Path,
    ) -> Result<(), Error> {
        let mut uniforms = UniformsStorageVec::new();
        uniforms.push(
            "time".to_string(),
            (time.num_nanoseconds().unwrap() as f32) / 1000_000_000.0 % 4096.0,
        );
        uniforms.push("pointer".to_string(), pointer);

        for name in &self.order {
            if name == "__default__" {
                self.nodes
                    .get_mut(name)
                    .unwrap()
                    .render_to_file(&mut uniforms, path)?;
            } else {
                let output = self.nodes
                    .get_mut(name)
                    .unwrap()
                    .render(&mut uniforms)?;
            }
        }

        Ok(())
    }
}
