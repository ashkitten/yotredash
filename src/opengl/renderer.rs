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
use config::Config;
use config::nodes::{NodeConfig, NodeParameter};
use super::nodes::*;

/// An implementation of a `Renderer` which uses OpenGL
pub struct OpenGLRenderer {
    /// The facade it uses to render
    facade: Rc<Facade>,
    /// Maps names to nodes
    nodes: HashMap<String, Box<Node>>,
    /// Node configurations for mapping outputs to inputs
    node_configs: HashMap<String, NodeConfig>,
    /// Order to render nodes in
    order: Vec<String>,
}

fn init_nodes(
    config: &Config,
    facade: &Rc<Facade>,
) -> Result<(HashMap<String, Box<Node>>, Vec<String>), Error> {
    ensure!(
        config.nodes.contains_key("__default__"),
        "Config does not contain node __default__"
    );

    let mut nodes: HashMap<String, Box<Node>> = HashMap::new();
    let mut dep_graph: DepGraph<&str> = DepGraph::new();
    dep_graph.register_node("__default__");

    for (name, node_config) in &config.nodes {
        match *node_config {
            NodeConfig::Image(ref image_config) => {
                let mut image_config = image_config.clone();
                image_config.path = config.path_to(&image_config.path);

                nodes.insert(
                    name.to_string(),
                    Box::new(ImageNode::new(&facade, image_config)?),
                );
            }

            NodeConfig::Shader(ref shader_config) => {
                {
                    // Replace the paths with absolute paths
                    let mut shader_config = shader_config.clone();
                    shader_config.vertex = config.path_to(&shader_config.vertex);
                    shader_config.fragment = config.path_to(&shader_config.fragment);

                    nodes.insert(
                        name.to_string(),
                        Box::new(ShaderNode::new(&facade, shader_config)?),
                    );
                }

                dep_graph.register_dependencies(
                    name,
                    shader_config
                        .textures
                        .iter()
                        .filter_map(|texture| match texture {
                            &NodeParameter::NodeConnection { ref node } => Some(node.as_str()),
                            &NodeParameter::Static(_) => None,
                        })
                        .collect(),
                );
            }

            NodeConfig::Blend(ref blend_config) => {
                nodes.insert(
                    name.to_string(),
                    Box::new(BlendNode::new(&facade, blend_config.clone())?),
                );

                dep_graph.register_dependencies(
                    name,
                    blend_config
                        .textures
                        .iter()
                        .filter_map(|texture| match texture {
                            &NodeParameter::NodeConnection { ref node } => Some(node.as_str()),
                            &NodeParameter::Static(_) => None,
                        })
                        .collect(),
                );
            }

            // TODO: Color in a better format
            NodeConfig::Text(ref text_config) => {
                nodes.insert(
                    name.to_string(),
                    Box::new(TextNode::new(&facade, text_config.clone())?),
                );
            }

            NodeConfig::Fps(ref fps_config) => {
                nodes.insert(
                    name.to_string(),
                    Box::new(FpsNode::new(&facade, fps_config.clone())?),
                );
            }
        }
    }

    let mut order = Vec::new();
    for node in dep_graph.dependencies_of(&"__default__")? {
        order.push(node?.to_string());
    }
    debug!("Render order: {}", order.join(", "));

    let dangling_nodes: Vec<String> = nodes
        .keys()
        .filter(|name| !order.contains(name))
        .cloned()
        .collect();
    if dangling_nodes.len() == 1 {
        warn!("Dangling node: {}", dangling_nodes[0]);
    } else if dangling_nodes.len() > 1 {
        warn!("Dangling nodes: {}", dangling_nodes.join(", "));
    }

    Ok((nodes, order))
}

fn map_node_io(
    config: &NodeConfig,
    outputs: &HashMap<String, NodeOutputs>,
    time: f32,
    pointer: [f32; 4],
) -> Result<NodeInputs, Error> {
    Ok(match config {
        &NodeConfig::Image(_) => NodeInputs::Image,

        &NodeConfig::Shader(ref shader_config) => {
            let mut textures = HashMap::new();
            for texture in &shader_config.textures {
                match texture {
                    &NodeParameter::NodeConnection { ref node } => {
                        match &outputs[node] {
                            &NodeOutputs::Texture2d(ref texture) => {
                                textures.insert(node.to_string(), Rc::clone(texture))
                            }
                            ref other => bail!("Wrong input type `{:?}` for `textures`", other),
                        };
                    }
                    &NodeParameter::Static(_) => (),
                }
            }
            NodeInputs::Shader {
                time,
                pointer,
                textures,
            }
        }

        &NodeConfig::Blend(ref blend_config) => {
            let mut textures = HashMap::new();
            for texture in &blend_config.textures {
                match texture {
                    &NodeParameter::NodeConnection { ref node } => {
                        match &outputs[node] {
                            &NodeOutputs::Texture2d(ref texture) => {
                                textures.insert(node.to_string(), Rc::clone(texture))
                            }
                            ref other => bail!("Wrong input type `{:?}` for `textures`", other),
                        };
                    }
                    &NodeParameter::Static(_) => (),
                }
            }
            NodeInputs::Blend { textures }
        }

        &NodeConfig::Text(ref text_config) => {
            let text = match &text_config.text {
                &NodeParameter::NodeConnection { ref node } => match &outputs[node] {
                    &NodeOutputs::Text(ref text) => Some(text.to_string()),
                    ref other => bail!("Wrong input type `{:?}` for `text`", other),
                },
                &NodeParameter::Static(_) => None,
            };
            let position = match &text_config.position {
                &NodeParameter::NodeConnection { ref node } => match &outputs[node] {
                    &NodeOutputs::Float2(ref position) => Some(position.clone()),
                    ref other => bail!("Wrong input type `{:?}` for `position`", other),
                },
                &NodeParameter::Static(_) => None,
            };
            let color = match &text_config.color {
                &NodeParameter::NodeConnection { ref node } => match &outputs[node] {
                    &NodeOutputs::Color(ref color) => Some(color.clone()),
                    ref other => bail!("Wrong input type `{:?}` for `position`", other),
                },
                &NodeParameter::Static(_) => None,
            };

            NodeInputs::Text {
                text,
                position,
                color,
            }
        }

        &NodeConfig::Fps(ref fps_config) => {
            let position = match &fps_config.position {
                &NodeParameter::NodeConnection { ref node } => match &outputs[node] {
                    &NodeOutputs::Float2(ref position) => Some(position.clone()),
                    ref other => bail!("Wrong input type `{:?}` for `position`", other),
                },
                &NodeParameter::Static(_) => None,
            };
            let color = match &fps_config.color {
                &NodeParameter::NodeConnection { ref node } => match &outputs[node] {
                    &NodeOutputs::Color(ref color) => Some(color.clone()),
                    ref other => bail!("Wrong input type `{:?}` for `position`", other),
                },
                &NodeParameter::Static(_) => None,
            };

            NodeInputs::Fps { position, color }
        }
    })
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
            node_configs: config.nodes,
            order,
        })
    }

    fn render(&mut self, time: ::time::Duration, pointer: [f32; 4]) -> Result<(), Error> {
        let height = self.facade.get_context().get_framebuffer_dimensions().1;
        let time = (time.num_nanoseconds().unwrap() as f32) / 1000_000_000.0 % 4096.0;
        let pointer = [
            pointer[0],
            height as f32 - pointer[1],
            pointer[2],
            height as f32 - pointer[3],
        ];

        let mut outputs: HashMap<String, NodeOutputs> = HashMap::new();

        for name in &self.order {
            let inputs = map_node_io(&self.node_configs[name], &outputs, time, pointer)?;

            if name == "__default__" {
                self.nodes.get_mut(name).unwrap().present(&inputs)?;
            } else {
                outputs.insert(
                    name.to_string(),
                    self.nodes.get_mut(name).unwrap().render(&inputs)?,
                );
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
        let height = self.facade.get_context().get_framebuffer_dimensions().1;
        let time = (time.num_nanoseconds().unwrap() as f32) / 1000_000_000.0 % 4096.0;
        let pointer = [
            pointer[0],
            height as f32 - pointer[1],
            pointer[2],
            height as f32 - pointer[3],
        ];

        let mut outputs: HashMap<String, NodeOutputs> = HashMap::new();

        for name in &self.order {
            let inputs = map_node_io(&self.node_configs[name], &outputs, time, pointer)?;

            if name == "__default__" {
                self.nodes
                    .get_mut(name)
                    .unwrap()
                    .render_to_file(&inputs, path)?;
            } else {
                outputs.insert(
                    name.to_string(),
                    self.nodes.get_mut(name).unwrap().render(&inputs)?,
                );
            }
        }

        Ok(())
    }
}
