//! An implementation of `Renderer` using OpenGL

use failure::{bail, ensure, format_err, Error, ResultExt, SyncFailure};
use glium::{
    backend::{
        glutin::{headless::Headless, Display},
        Facade,
    },
    glutin::{Context, ContextBuilder, WindowBuilder},
    texture::{MipmapsOption, RawImage2d, Texture2d},
    uniforms::MagnifySamplerFilter,
    BlitTarget, Rect, Surface,
};
use image;
use log::{debug, warn};
use solvent::DepGraph;
use std::{
    collections::HashMap,
    rc::Rc,
    sync::mpsc::{self, Receiver, Sender},
};
use winit::EventsLoop;

use super::{nodes::*, text::TextRenderer};
use crate::{
    config::{
        nodes::{NodeConfig, NodeConnection, NodeParameter},
        Config,
    },
    event::RendererEvent,
    DebugRenderer, Renderer,
};

type NodeMap = HashMap<String, NodeType>;
type NodeConfigMap = HashMap<String, NodeConfig>;

/// An implementation of a `Renderer` which uses OpenGL
pub struct OpenGLRenderer {
    /// The facade it uses to render
    facade: Rc<dyn Facade>,
    /// Maps names to nodes
    nodes: NodeMap,
    /// Node configurations for mapping outputs to inputs
    node_configs: NodeConfigMap,
    /// Order to render nodes in
    order: Vec<String>,
    /// Receiver for events
    receiver: Receiver<RendererEvent>,
    /// Sender for pointer events
    senders: Vec<Sender<RendererEvent>>,
}

fn init_nodes(
    config: &Config,
    facade: &Rc<dyn Facade>,
) -> Result<(NodeMap, Vec<String>, Vec<Sender<RendererEvent>>), Error> {
    let mut senders = Vec::new();

    let mut nodes: NodeMap = HashMap::new();
    let mut dep_graph: DepGraph<&str> = DepGraph::new();
    let mut output_node = "";

    for (name, node_config) in &config.nodes {
        match *node_config {
            NodeConfig::Info => {
                let (sender, receiver) = mpsc::channel();
                senders.push(sender);

                let (width, height) = facade.get_context().get_framebuffer_dimensions();

                nodes.insert(
                    name.to_string(),
                    NodeType::Info(InfoNode::new(receiver, [width as f32, height as f32])),
                );
            }

            NodeConfig::Output(ref output_config) => {
                nodes.insert(name.to_string(), NodeType::Output(OutputNode::new(facade)?));

                dep_graph.register_dependency(name, &output_config.texture.node);

                ensure!(output_node.is_empty(), "There can only be one output node");
                output_node = name;
            }

            NodeConfig::Image(ref image_config) => {
                let mut image_config = image_config.clone();
                image_config.path = config.path_to(&image_config.path);

                nodes.insert(
                    name.to_string(),
                    NodeType::Image(ImageNode::new(facade, image_config)?),
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
                        NodeType::Shader(ShaderNode::new(facade, shader_config)?),
                    );
                }

                dep_graph.register_dependencies(
                    name,
                    shader_config
                        .uniforms
                        .iter()
                        .map(|connection| connection.node.as_str())
                        .collect(),
                );
            }

            NodeConfig::Blend(ref blend_config) => {
                let (sender, receiver) = mpsc::channel();
                senders.push(sender);

                nodes.insert(
                    name.to_string(),
                    NodeType::Blend(BlendNode::new(facade, blend_config, receiver)?),
                );

                dep_graph.register_dependencies(
                    name,
                    blend_config
                        .textures
                        .iter()
                        .map(|connection| connection.node.as_str())
                        .collect(),
                );
            }

            // TODO: Color in a better format
            NodeConfig::Text(ref text_config) => {
                let (sender, receiver) = mpsc::channel();
                senders.push(sender);

                nodes.insert(
                    name.to_string(),
                    NodeType::Text(TextNode::new(facade, text_config.clone(), receiver)?),
                );
            }

            NodeConfig::Fps(ref fps_config) => {
                let (sender, receiver) = mpsc::channel();
                senders.push(sender);

                nodes.insert(
                    name.to_string(),
                    NodeType::Fps(FpsNode::new(facade, fps_config.clone(), receiver)?),
                );
            }

            NodeConfig::Audio => {
                nodes.insert(name.to_string(), NodeType::Audio(AudioNode::new(facade)?));
            }

            NodeConfig::Feedback(ref feedback_config) => {
                nodes.insert(
                    name.to_string(),
                    NodeType::Feedback(FeedbackNode::new(facade, feedback_config.clone())?),
                );
            }
        }
    }

    ensure!(!output_node.is_empty(), "No output node specified");

    let mut order = Vec::new();
    for node in dep_graph.dependencies_of(&output_node)? {
        order.push(node?.to_string());
    }
    debug!("Render order: {}", order.join(", "));

    let dangling_nodes: Vec<String> = nodes
        .keys()
        .filter(|name| !order.contains(name))
        .cloned()
        .collect();
    if dangling_nodes.len() == 1 {
        warn!("Dangling node: `{}`", dangling_nodes[0]);
    } else if dangling_nodes.len() > 1 {
        warn!("Dangling nodes: `{}`", dangling_nodes.join(", "));
    }

    Ok((nodes, order, senders))
}

fn map_node_io(
    config: &NodeConfig,
    outputs: &HashMap<String, HashMap<String, NodeOutput>>,
) -> Result<NodeInputs, Error> {
    let get_node_output = |connection: &NodeConnection| -> Result<_, Error> {
        Ok(outputs
            .get(&connection.node)
            .ok_or_else(|| format_err!("No such node: `{}`", connection.node))?
            .get(&connection.output)
            .ok_or_else(|| {
                format_err!(
                    "No such output on node `{}`: `{}`",
                    connection.node,
                    connection.output
                )
            })?)
    };

    Ok(match *config {
        NodeConfig::Info => NodeInputs::Info,

        NodeConfig::Output(ref output_config) => match *get_node_output(&output_config.texture)? {
            NodeOutput::Texture2d(ref texture) => NodeInputs::Output {
                texture: Rc::clone(texture),
            },
            _ => bail!("Wrong input type for `texture`"),
        },

        NodeConfig::Image(_) => NodeInputs::Image,

        NodeConfig::Shader(ref shader_config) => {
            let mut uniforms = HashMap::new();
            for connection in &shader_config.uniforms {
                uniforms.insert(connection.clone(), get_node_output(connection)?.clone());
            }
            NodeInputs::Shader { uniforms }
        }

        NodeConfig::Blend(ref blend_config) => {
            let mut textures = Vec::new();
            for connection in &blend_config.textures {
                match *get_node_output(connection)? {
                    NodeOutput::Texture2d(ref texture) => textures.push(Rc::clone(texture)),
                    _ => bail!("Wrong input type for `uniforms`"),
                };
            }
            NodeInputs::Blend { textures }
        }

        NodeConfig::Text(ref text_config) => {
            let text = match text_config.text {
                NodeParameter::NodeConnection(ref connection) => {
                    match *get_node_output(connection)? {
                        NodeOutput::Text(ref text) => Some(text.to_string()),
                        _ => bail!("Wrong input type for `text`"),
                    }
                }
                NodeParameter::Static(_) => None,
            };
            let position = match text_config.position {
                NodeParameter::NodeConnection(ref connection) => {
                    match *get_node_output(connection)? {
                        NodeOutput::Float2(ref position) => Some(*position),
                        _ => bail!("Wrong input type for `position`"),
                    }
                }
                NodeParameter::Static(_) => None,
            };
            let color = match text_config.color {
                NodeParameter::NodeConnection(ref connection) => {
                    match *get_node_output(connection)? {
                        NodeOutput::Color(ref color) => Some(*color),
                        _ => bail!("Wrong input type for `position`"),
                    }
                }
                NodeParameter::Static(_) => None,
            };

            NodeInputs::Text {
                text,
                position,
                color,
            }
        }

        NodeConfig::Fps(ref fps_config) => {
            let position = match fps_config.position {
                NodeParameter::NodeConnection(ref connection) => {
                    match *get_node_output(connection)? {
                        NodeOutput::Float2(ref position) => Some(*position),
                        _ => bail!("Wrong input type for `position`"),
                    }
                }
                NodeParameter::Static(_) => None,
            };
            let color = match fps_config.color {
                NodeParameter::NodeConnection(ref connection) => {
                    match *get_node_output(connection)? {
                        NodeOutput::Color(ref color) => Some(*color),
                        _ => bail!("Wrong input type for `position`"),
                    }
                }
                NodeParameter::Static(_) => None,
            };

            NodeInputs::Fps { position, color }
        }

        NodeConfig::Audio => NodeInputs::Audio,

        NodeConfig::Feedback(_) => NodeInputs::Feedback,
    })
}

impl OpenGLRenderer {
    /// Create a new instance on an existing Facade
    pub fn new(
        config: &Config,
        facade: &Rc<dyn Facade>,
        receiver: Receiver<RendererEvent>,
    ) -> Result<Self, Error> {
        debug!(
            "OpenGL backend: {}",
            facade.get_context().get_opengl_version_string()
        );

        let (nodes, order, senders) = init_nodes(config, facade)?;

        Ok(Self {
            facade: Rc::clone(facade),
            nodes,
            node_configs: config.nodes.clone(),
            order,
            receiver,
            senders,
        })
    }
}

impl Renderer for OpenGLRenderer {
    fn update(&mut self) -> Result<(), Error> {
        while let Ok(event) = self.receiver.try_recv() {
            match event {
                RendererEvent::Capture(path) => {
                    let (width, height) = self.facade.get_context().get_framebuffer_dimensions();
                    let texture = Texture2d::empty_with_mipmaps(
                        &*self.facade,
                        MipmapsOption::NoMipmap,
                        width,
                        height,
                    )?;

                    let source_rect = Rect {
                        left: 0,
                        bottom: 0,
                        width,
                        height,
                    };

                    let target_rect = BlitTarget {
                        left: 0,
                        bottom: height,
                        width: width as i32,
                        height: -(height as i32),
                    };

                    texture.as_surface().blit_from_frame(
                        &source_rect,
                        &target_rect,
                        MagnifySamplerFilter::Nearest,
                    );

                    let raw: RawImage2d<'_, u8> = texture.read();
                    image::save_buffer(path, &raw.data, raw.width, raw.height, image::RGBA(8))?;
                }

                event => {
                    for sender in &self.senders {
                        sender.send(event.clone())?;
                    }
                }
            }
        }

        Ok(())
    }

    fn render(&mut self) -> Result<(), Error> {
        let mut outputs: HashMap<String, HashMap<String, NodeOutput>> = HashMap::new();

        let mut feedback_nodes = Vec::new();

        for name in &self.order {
            ensure!(
                self.node_configs.contains_key(name),
                "No such node: `{}`",
                name
            );

            let inputs = map_node_io(&self.node_configs[name], &outputs)
                .context(format!("Error on node `{}`", name))?;

            outputs.insert(
                name.to_string(),
                self.nodes.get_mut(name).unwrap().render(&inputs)?,
            );

            if let NodeType::Feedback(_) = self.nodes[name] {
                feedback_nodes.push(name);
            }
        }

        for name in feedback_nodes {
            if let &mut NodeType::Feedback(ref mut node) = self.nodes.get_mut(name).unwrap() {
                let mut inputs = HashMap::new();
                if let &NodeConfig::Feedback(ref feedback_config) = &self.node_configs[name] {
                    for connection in &feedback_config.inputs {
                        inputs.insert(
                            connection.clone(),
                            outputs
                                .get(&connection.node)
                                .ok_or_else(|| format_err!("No such node: `{}`", connection.node))?
                                .get(&connection.output)
                                .ok_or_else(|| {
                                    format_err!(
                                        "No such output on node `{}`: `{}`",
                                        connection.node,
                                        connection.output
                                    )
                                })?
                                .clone(),
                        );
                    }
                }
                node.update(&inputs);
            }
        }

        Ok(())
    }

    fn swap_buffers(&self) -> Result<(), Error> {
        self.facade.get_context().swap_buffers()?;
        Ok(())
    }
}

/// Renders errors
pub struct OpenGLDebugRenderer {
    /// Facade for interacting with OpenGL
    facade: Rc<dyn Facade>,
    /// `TextRenderer` for displaying errors
    error_renderer: TextRenderer,
}

impl OpenGLDebugRenderer {
    /// Create a new instance
    pub fn new(facade: &Rc<dyn Facade>) -> Result<Self, Error> {
        Ok(Self {
            facade: Rc::clone(facade),
            error_renderer: TextRenderer::new(facade, "", 20.0)?,
        })
    }
}

impl DebugRenderer for OpenGLDebugRenderer {
    fn draw_error(&mut self, error: &Error) -> Result<(), Error> {
        let mut target = self.facade.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);
        self.error_renderer.draw_text(
            &mut target,
            &crate::format_error(error),
            [0.0, 0.0],
            [1.0, 0.3, 0.3, 1.0],
        )?;
        target.finish()?;

        Ok(())
    }
}

/// Create an appropriate Facade
pub fn new_facade(config: &Config, events_loop: &EventsLoop) -> Result<Rc<dyn Facade>, Error> {
    if !config.headless {
        let window_builder = WindowBuilder::new()
            .with_dimensions((config.width, config.height).into())
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
        let display =
            Display::new(window_builder, context_builder, events_loop).map_err(SyncFailure::new)?;
        crate::platform::window::init(display.gl_window().window(), &config);

        Ok(Rc::new(display))
    } else {
        let context_builder = ContextBuilder::new();
        let context = Context::new(&events_loop, context_builder, false).unwrap();
        Ok(Rc::new(Headless::new(context)?))
    }
}
