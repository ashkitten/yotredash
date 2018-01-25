//! The `config` module provides definitions for all configuration structs as well as methods
//! necessary for configuration via yaml and command line.

use clap::{App, Arg, ArgMatches};
use nfd::{self, Response};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use failure::Error;
use failure::ResultExt;

use platform::config::PlatformSpecificConfig;

/// Blend node operations
#[derive(Debug, Deserialize, Clone)]
#[allow(non_camel_case_types)]
pub enum BlendOp {
    /// Take the minimum RGBA value
    min,
    /// Take the maximum RGBA value
    max,
    /// Add the RGBA values
    add,
    /// Subtract the RGBA values
    sub,
}

/// The node configuration contains all the information necessary to build a node
#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
#[serde(deny_unknown_fields)]
#[allow(non_camel_case_types)]
pub enum NodeConfig {
    /// Image node type
    image {
        /// Relative path to the image
        path: PathBuf,
    },

    /// Shader node type
    shader {
        /// Relative path to the vertex shader
        vertex: PathBuf,

        /// Relative path to the fragment shader
        fragment: PathBuf,

        /// Input nodes for the shader program
        #[serde(default)]
        inputs: Vec<String>,
    },

    /// Blend node type - blends the output of multiple nodes
    blend {
        /// Math operation
        operation: BlendOp,

        /// Input node names and alpha transparencies
        inputs: Vec<String>,
    },

    /// Text node type - renders text
    text {
        /// Text to render
        text: String,

        /// Position to render at
        #[serde(default)]
        position: [f32; 2],

        /// Color to render in
        #[serde(default = "text_default_color")]
        color: [f32; 4],

        /// Font name
        #[serde(default)]
        font_name: String,

        /// Font size
        #[serde(default = "text_default_font_size")]
        font_size: f32,
    },

    /// FPS counter node type - renders text
    fps {
        /// Position to render at
        #[serde(default)]
        position: [f32; 2],

        /// Color to render in
        #[serde(default = "text_default_color")]
        color: [f32; 4],

        /// Font name
        #[serde(default)]
        font_name: String,

        /// Font size
        #[serde(default = "text_default_font_size")]
        font_size: f32,

        /// Update interval (seconds)
        #[serde(default = "fps_default_interval")]
        interval: f32,
    },
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

/// The main configuration contains all the information necessary to build a renderer
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The current working directory
    /// Not meant to actually be specified in yaml, but can be
    #[serde(default)]
    pub _cwd: PathBuf,

    /// The node configurations, keyed by name
    /// The node called `__default__` must be specified, as this is the output node
    #[serde(default)]
    pub nodes: HashMap<String, NodeConfig>,

    /// Initial width of the window
    #[serde(default = "default_width")]
    pub width: u32,

    /// Initial height of the window
    #[serde(default = "default_height")]
    pub height: u32,

    /// Whether or not to maximize the window
    #[serde(default = "default_maximize")]
    pub maximize: bool,

    /// Whether or not to make the window fullscreen
    #[serde(default = "default_fullscreen")]
    pub fullscreen: bool,

    /// Whether or not the program should use vertical sync
    #[serde(default = "default_vsync")]
    pub vsync: bool,

    /// Specifies which renderer to use (current options: opengl)
    #[serde(default = "default_renderer")]
    pub renderer: String,

    /// Use a headless renderer
    #[serde(default = "default_headless")]
    pub headless: bool,

    /// Reload automatically when file changes are detected
    #[serde(default = "default_autoreload")]
    pub autoreload: bool,

    /// Extra platform-specific configurations
    #[serde(default)]
    pub platform_config: PlatformSpecificConfig,
}

/// A function that returns the default value of the `width` field
fn default_width() -> u32 {
    640
}

/// A function that returns the default value of the `width` field
fn default_height() -> u32 {
    400
}

/// A function that returns the default value of the `maximize` field
fn default_maximize() -> bool {
    false
}

/// A function that returns the default value of the `fullscreen` field
fn default_fullscreen() -> bool {
    false
}

/// A function that returns the default value of the `vsync` field
fn default_vsync() -> bool {
    false
}

/// A function that returns the default value of the `renderer` field
fn default_renderer() -> String {
    "opengl".to_string()
}

/// A function that returns the default value of the `headless` field
fn default_headless() -> bool {
    false
}

/// A function that returns the default value of the `autoreload` field
fn default_autoreload() -> bool {
    false
}

impl Config {
    /// Builds the application description needed to parse command-line arguments
    pub fn build_cli() -> App<'static, 'static> {
        App::new("yotredash")
            .version("0.1.0")
            .author("Ash Levy <ashlea@protonmail.com>")
            .args(&[
                Arg::with_name("width")
                    .short("w")
                    .long("width")
                    .help("Set window width")
                    .takes_value(true),
                Arg::with_name("height")
                    .short("h")
                    .long("height")
                    .help("Set window height")
                    .takes_value(true),
                Arg::with_name("maximize")
                    .long("maximize")
                    .help("Maximize window dimensions"),
                Arg::with_name("fullscreen")
                    .long("fullscreen")
                    .help("Make window fullscreen"),
                Arg::with_name("vsync")
                    .long("vsync")
                    .help("Enable vertical sync"),
                Arg::with_name("renderer")
                    .long("renderer")
                    .help("Specify renderer to use")
                    .takes_value(true),
                Arg::with_name("headless")
                    .long("headless")
                    .help("Use a headless renderer - note that this will force the use of the Mesa OpenGL driver"),
                Arg::with_name("autoreload")
                    .long("autoreload")
                    .help("Automatically reload when changes to the shaders are detected"),
                Arg::with_name("config")
                    .short("c")
                    .long("config")
                    .help("Load a config file")
                    .takes_value(true),
            ])
            .after_help(
                "\
                 This program uses `env_logger` as its logging backend.\n\
                 Common usage: `RUST_LOG=yotredash=info yotredash`\n\
                 See http://rust-lang-nursery.github.io/log/env_logger/ for more information.\
                 ",
            )
    }

    /// Parses the configuration from command-line arguments
    fn merge_args(&mut self, args: &ArgMatches) -> Result<(), Error> {
        if let Some(value) = args.value_of("width") {
            self.width = value.parse::<u32>()?;
        }

        if let Some(value) = args.value_of("height") {
            self.height = value.parse::<u32>()?;
        }

        if args.is_present("maximize") {
            self.maximize = true;
        }

        if args.is_present("fullscreen") {
            self.fullscreen = true;
        }

        if args.is_present("vsync") {
            self.vsync = true;
        }

        if let Some(value) = args.value_of("renderer") {
            self.renderer = value.to_string();
        }

        if args.is_present("headless") {
            self.headless = true;
        }

        if args.is_present("autoreload") {
            self.autoreload = true;
        }

        Ok(())
    }

    /// Parses the configuration from a specified file
    fn from_file(path: &Path) -> Result<Self, Error> {
        debug!("Using config file: {}", path.to_str().unwrap());
        let file = File::open(path).context("Unable to open config file")?;
        let mut reader = BufReader::new(file);
        let mut config_str = String::new();
        reader
            .read_to_string(&mut config_str)
            .context("Could not read config file")?;
        let mut config: Config = ::serde_yaml::from_str(&config_str)?;

        config._cwd = path.parent().unwrap().to_path_buf();

        Ok(config)
    }

    /// Returns the configuration, appropriately noded from both command-line arguments and the
    /// config file
    pub fn parse(path: &Path) -> Result<Self, Error> {
        let app = PlatformSpecificConfig::build_cli();
        let args = app.get_matches();

        let mut config = Self::from_file(path)?;
        config.merge_args(&args)?;

        Ok(config)
    }

    /// Returns the chosen config file path
    pub fn get_path() -> Result<PathBuf, Error> {
        let app = PlatformSpecificConfig::build_cli();
        let args = app.get_matches();

        let path = match args.value_of("config") {
            Some(path) => Path::new(&path).to_path_buf(),
            None => {
                let result = nfd::open_file_dialog(
                    Some("yml,yaml,json"),
                    ::std::env::current_dir().unwrap_or_default().to_str(),
                )?;
                match result {
                    Response::Okay(path) => Path::new(&path).to_path_buf(),
                    Response::OkayMultiple(paths) => Path::new(&paths[0]).to_path_buf(),
                    Response::Cancel => bail!("No file selected"),
                }
            }
        };

        Ok(path)
    }

    /// Provides a way to get the complete path to a file referenced in a configuration
    pub fn path_to(&self, path: &Path) -> PathBuf {
        self._cwd.join(path)
    }
}
