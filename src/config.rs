//! The `config` module provides definitions for all configuration structs as well as methods
//! necessary for configuration via yaml and command line.

/// The source configuration contains all the information necessary to build a source
pub mod source_config {
    use std::path::PathBuf;

    /// The source configuration contains all the information necessary to build a source
    #[derive(Deserialize, Clone)]
    pub struct SourceConfig {
        /// The path to the source file (relative to the configuration file, if there is one)
        pub path: PathBuf,
        /// The kind of the source file (image, etc) if applicable
        pub kind: String,
    }
}

/// The buffer configuration contains all the information necessary to build a buffer
pub mod buffer_config {
    use std::path::{Path, PathBuf};

    /// The buffer configuration contains all the information necessary to build a buffer
    #[derive(Deserialize, Clone)]
    pub struct BufferConfig {
        /// The current working directory relative to the main config file
        #[serde(default)]
        pub _cwd: PathBuf,

        /// The path to the vertex shader (relative to the configuration file, if there is one)
        pub vertex: PathBuf,

        /// The path to the fragment shader (relative to the configuration file, if there is one)
        pub fragment: PathBuf,

        /// The names of the source configurations this buffer references
        #[serde(default = "default_sources")]
        pub sources: Vec<String>,

        /// The width of the buffer
        #[serde(default = "default_width")]
        pub width: u32,

        /// The height of the buffer
        #[serde(default = "default_height")]
        pub height: u32,

        /// The names of the buffer configurations this buffer references
        #[serde(default = "default_depends")]
        pub depends: Vec<String>,

        /// Whether or not this buffer is resizeable
        #[serde(default = "default_resizeable")]
        pub resizeable: bool,
    }

    /// A function that returns the default value of the `sources` field
    pub fn default_sources() -> Vec<String> {
        Vec::new()
    }

    /// A function that returns the default value of the `width` field
    pub fn default_width() -> u32 {
        640
    }

    /// A function that returns the default value of the `height` field
    pub fn default_height() -> u32 {
        400
    }

    /// A function that returns the default value of the `depends` field
    pub fn default_depends() -> Vec<String> {
        Vec::new()
    }

    /// A function that returns the default value of the `resizeable` field
    pub fn default_resizeable() -> bool {
        true
    }

    impl BufferConfig {
        /// Provides a way to get the complete path to a file referenced in a configuration
        pub fn path_to(&self, path: &Path) -> PathBuf {
            self._cwd.join(path)
        }
    }
}

use clap::{App, Arg, ArgMatches};
use nfd::{self, Response};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use failure::Error;
use failure::ResultExt;

use self::buffer_config::BufferConfig;
use self::source_config::SourceConfig;
use platform::config::PlatformSpecificConfig;

/// The main configuration contains all the information necessary to build a renderer
#[derive(Deserialize, Clone)]
pub struct Config {
    /// The current working directory
    /// Not meant to actually be specified in yaml, but can be
    #[serde(default)]
    pub _cwd: PathBuf,

    /// The buffer configurations, keyed by name
    ///
    /// The buffer called `__default__` must be specified, as this is the output buffer
    pub buffers: HashMap<String, BufferConfig>,

    /// The source configurations, keyed by name
    #[serde(default = "default_sources")]
    pub sources: HashMap<String, SourceConfig>,

    /// Whether or not to maximize the window
    #[serde(default = "default_maximize")]
    pub maximize: bool,

    /// Whether or not to make the window fullscreen
    #[serde(default = "default_fullscreen")]
    pub fullscreen: bool,

    /// Whether or not the program should use vertical sync
    #[serde(default = "default_vsync")]
    pub vsync: bool,

    /// Whether or not to show the FPS counter
    #[serde(default = "default_fps")]
    pub fps: bool,

    /// The name of the font to use
    #[serde(default = "default_font")]
    pub font: String,

    /// The size of the font, in points
    #[serde(default = "default_font_size")]
    pub font_size: f32,

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

/// A function that returns the default value of the `sources` field
fn default_sources() -> HashMap<String, SourceConfig> {
    HashMap::new()
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

/// A function that returns the default value of the `fps` field
fn default_fps() -> bool {
    false
}

/// A function that returns the default value of the `font` field
fn default_font() -> String {
    "".to_string()
}

/// A function that returns the default value of the `font` field
fn default_font_size() -> f32 {
    20.0
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
                Arg::with_name("fps")
                    .long("fps")
                    .help("Enable FPS log to console"),
                Arg::with_name("font")
                    .long("font")
                    .help("Specify font")
                    .takes_value(true),
                Arg::with_name("font_size")
                    .long("font-size")
                    .help("Specify font size")
                    .takes_value(true),
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
            self.buffers.get_mut("__default__").unwrap().width = value.parse::<u32>()?;
        }

        if let Some(value) = args.value_of("height") {
            self.buffers.get_mut("__default__").unwrap().height = value.parse::<u32>()?;
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

        if args.is_present("fps") {
            self.fps = true;
        }

        if let Some(value) = args.value_of("font") {
            self.font = value.to_string();
        }

        if let Some(value) = args.value_of("font_size") {
            self.font_size = value.parse::<f32>()?;
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
        info!("Using config file: {}", path.to_str().unwrap());
        let file = File::open(path).context("Unable to open config file")?;
        let mut reader = BufReader::new(file);
        let mut config_str = String::new();
        reader
            .read_to_string(&mut config_str)
            .context("Could not read config file")?;
        let mut config: Config = ::serde_yaml::from_str(&config_str)?;

        config._cwd = path.parent().unwrap().to_path_buf();
        for buffer in config.buffers.values_mut() {
            buffer._cwd = config._cwd.clone();
        }

        Ok(config)
    }

    /// Returns the configuration, appropriately sourced from both command-line arguments and the
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
