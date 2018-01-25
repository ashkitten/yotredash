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
use super::nodes::*;

/// An implementation of a `Renderer` which uses OpenGL
pub struct OpenGLRenderer {
    /// The facade it uses to render
    facade: Rc<Facade>,
    /// Maps names to nodes
    nodes: HashMap<String, Box<Node>>,
    /// Order to render nodes in
    order: Vec<String>,
}

fn init_nodes(
    config: &Config,
    facade: &Rc<Facade>,
) -> Result<(HashMap<String, Box<Node>>, Vec<String>), Error> {
    let mut nodes: HashMap<String, Box<Node>> = HashMap::new();
    let mut dep_graph: DepGraph<&str> = DepGraph::new();
    dep_graph.register_node("__default__");

    for (name, node_config) in config.nodes.iter() {
        debug!("Node '{}': {:?}", name, node_config);

        match *node_config {
            NodeConfig::image { ref path } => {
                nodes.insert(
                    name.to_string(),
                    Box::new(ImageNode::new(
                        &facade,
                        name.to_string(),
                        &config.path_to(path),
                    )?),
                );
            }

            NodeConfig::shader {
                ref vertex,
                ref fragment,
                ref inputs,
            } => {
                nodes.insert(
                    name.to_string(),
                    Box::new(ShaderNode::new(
                        &facade,
                        name.to_string(),
                        &config.path_to(vertex),
                        &config.path_to(fragment),
                    )?),
                );

                dep_graph.register_dependencies(
                    name,
                    inputs.iter().map(|input| input.as_str()).collect(),
                );
            }

            NodeConfig::blend {
                ref operation,
                ref inputs,
            } => {
                nodes.insert(
                    name.to_string(),
                    Box::new(BlendNode::new(
                        &facade,
                        name.to_string(),
                        operation.clone(),
                        inputs.clone(),
                    )?),
                );

                dep_graph.register_dependencies(
                    name,
                    inputs.iter().map(|input| input.as_str()).collect(),
                );
            }

            // TODO: Color in a better format
            NodeConfig::text {
                ref text,
                ref position,
                ref color,
                ref font_name,
                ref font_size,
            } => {
                nodes.insert(
                    name.to_string(),
                    Box::new(TextNode::new(
                        &facade,
                        name.to_string(),
                        text.to_string(),
                        position.clone(),
                        color.clone(),
                        font_name,
                        font_size.clone(),
                    )?),
                );
            }

            NodeConfig::fps {
                ref position,
                ref color,
                ref font_name,
                ref font_size,
                ref interval,
            } => {
                nodes.insert(
                    name.to_string(),
                    Box::new(FpsNode::new(
                        &facade,
                        name.to_string(),
                        position.clone(),
                        color.clone(),
                        font_name,
                        font_size.clone(),
                        interval.clone(),
                    )?),
                );
            }
        }
    }

    let mut order = Vec::new();
    for dep in dep_graph.dependencies_of(&"__default__")? {
        order.push(dep?.to_string());
    }

    debug!("Render order: {:?}", order);

    Ok((nodes, order))
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

        let (nodes, order) = init_nodes(&config, &facade)?;

        Ok(Self {
            facade,
            nodes,
            order,
        })
    }

    fn render(&mut self, time: ::time::Duration, pointer: [f32; 4]) -> Result<(), Error> {
        let (width, height) = self.facade.get_context().get_framebuffer_dimensions();
        let time = (time.num_nanoseconds().unwrap() as f32) / 1000_000_000.0 % 4096.0;
        let pointer = [
            pointer[0],
            height as f32 - pointer[1],
            pointer[2],
            height as f32 - pointer[3],
        ];

        let mut uniforms = UniformsStorageVec::new();
        uniforms.push("time", time);
        uniforms.push("pointer", pointer);
        uniforms.push("resolution", (width as f32, height as f32));

        for name in &self.order {
            if name == "__default__" {
                self.nodes.get_mut(name).unwrap().present(&mut uniforms)?;
            } else {
                self.nodes.get_mut(name).unwrap().render(&mut uniforms)?;
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

        let (nodes, order) = init_nodes(config, &self.facade)?;
        self.nodes = nodes;
        self.order = order;

        Ok(())
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<(), Error> {
        debug!("Resized window to {}x{}", width, height);

        for node in self.nodes.values_mut() {
            node.resize(width, height)?;
        }

        Ok(())
    }

    fn render_to_file(
        &mut self,
        time: ::time::Duration,
        pointer: [f32; 4],
        path: &Path,
    ) -> Result<(), Error> {
        let (width, height) = self.facade.get_context().get_framebuffer_dimensions();
        let time = (time.num_nanoseconds().unwrap() as f32) / 1000_000_000.0 % 4096.0;
        let pointer = [
            pointer[0],
            height as f32 - pointer[1],
            pointer[2],
            height as f32 - pointer[3],
        ];

        let mut uniforms = UniformsStorageVec::new();
        uniforms.push("time", time);
        uniforms.push("pointer", pointer);
        uniforms.push("resolution", (width as f32, height as f32));

        for name in &self.order {
            if name == "__default__" {
                self.nodes
                    .get_mut(name)
                    .unwrap()
                    .render_to_file(&mut uniforms, path)?;
            } else {
                self.nodes.get_mut(name).unwrap().render(&mut uniforms)?;
            }
        }

        Ok(())
    }
}
