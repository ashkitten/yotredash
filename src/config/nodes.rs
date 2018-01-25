use std::path::PathBuf;

/// Image node type
#[derive(Debug, Deserialize, Clone)]
pub struct ImageConfig {
    /// Relative path to the image
    pub path: PathBuf,
}

/// Shader node type
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ShaderConfig {
    /// Relative path to the vertex shader
    pub vertex: PathBuf,

    /// Relative path to the fragment shader
    pub fragment: PathBuf,

    /// Input nodes for the shader program
    #[serde(default)]
    pub inputs: Vec<String>,
}

/// Blend node type - blends the output of multiple nodes
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct BlendConfig {
    /// Math operation
    pub operation: BlendOp,

    /// Input node names and alpha transparencies
    pub inputs: Vec<String>,
}

/// Text node type - renders text
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct TextConfig {
    /// Text to render
    pub text: String,

    /// Position to render at
    #[serde(default)]
    pub position: [f32; 2],

    /// Color to render in
    #[serde(default = "text_default_color")]
    pub color: [f32; 4],

    /// Font name
    #[serde(default)]
    pub font_name: String,

    /// Font size
    #[serde(default = "text_default_font_size")]
    pub font_size: f32,
}

/// FPS counter node type - renders text
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct FpsConfig {
    /// Position to render at
    #[serde(default)]
    pub position: [f32; 2],

    /// Color to render in
    #[serde(default = "text_default_color")]
    pub color: [f32; 4],

    /// Font name
    #[serde(default)]
    pub font_name: String,

    /// Font size
    #[serde(default = "text_default_font_size")]
    pub font_size: f32,

    /// Update interval (seconds)
    #[serde(default = "fps_default_interval")]
    pub interval: f32,
}

/// Blend node operations
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum BlendOp {
    /// Take the minimum RGBA value
    Min,
    /// Take the maximum RGBA value
    Max,
    /// Add the RGBA values
    Add,
    /// Subtract the RGBA values
    Sub,
}

/// The node configuration contains all the information necessary to build a node
#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
pub enum NodeConfig {
    Image(ImageConfig),
    Shader(ShaderConfig),
    Blend(BlendConfig),
    Text(TextConfig),
    Fps(FpsConfig),
}

fn text_default_color() -> [f32; 4] {
    [1.0; 4]
}

fn text_default_font_size() -> f32 {
    20.0
}

fn fps_default_interval() -> f32 {
    1.0
}
