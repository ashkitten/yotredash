extern crate serde_yaml;

pub mod texture_config {
    #[derive(Deserialize, Clone)]
    pub struct TextureConfig {
        pub path: String,
    }
}

pub mod buffer_config {
    #[derive(Deserialize, Clone)]
    pub struct BufferConfig {
        pub vertex: String,
        pub fragment: String,
        #[serde(default = "default_textures")] pub textures: Vec<String>,
        #[serde(default = "default_width")] pub width: u32,
        #[serde(default = "default_height")] pub height: u32,
        #[serde(default = "default_depends")] pub depends: Vec<String>,
        #[serde(default = "default_resizeable")] pub resizeable: bool,
    }

    pub fn default_textures() -> Vec<String> {
        Vec::new()
    }

    pub fn default_width() -> u32 {
        640
    }

    pub fn default_height() -> u32 {
        400
    }

    pub fn default_depends() -> Vec<String> {
        Vec::new()
    }

    pub fn default_resizeable() -> bool {
        true
    }
}

use clap::{App, Arg, ArgMatches};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::path::Path;

use self::buffer_config::BufferConfig;
use self::texture_config::TextureConfig;
use errors::*;
use platform::config::PlatformSpecificConfig;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub buffers: HashMap<String, BufferConfig>,
    #[serde(default = "default_textures")] pub textures: HashMap<String, TextureConfig>,
    #[serde(default = "default_maximize")] pub maximize: bool,
    #[serde(default = "default_vsync")] pub vsync: bool,
    #[serde(default = "default_fps")] pub fps: bool,
    #[serde(default = "default_font")] pub font: String,
    #[serde(default = "default_renderer")] pub renderer: String,
    #[serde(default)] pub platform_config: PlatformSpecificConfig,
}

fn default_textures() -> HashMap<String, TextureConfig> {
    HashMap::new()
}

fn default_maximize() -> bool {
    false
}

fn default_vsync() -> bool {
    false
}

fn default_fps() -> bool {
    false
}

fn default_font() -> String {
    "".to_string()
}

fn default_renderer() -> String {
    "opengl".to_string()
}

impl Config {
    pub fn build_cli() -> App<'static, 'static> {
        App::new("yotredash")
            .version("0.1.0")
            .author("Ash Levy <ashlea@protonmail.com>")
            .args(&[
                Arg::with_name("vertex")
                    .short("v")
                    .long("vertex")
                    .help("Specify a vertex shader")
                    .takes_value(true),
                Arg::with_name("fragment")
                    .short("f")
                    .long("fragment")
                    .help("Specify a fragment shader")
                    .takes_value(true),
                Arg::with_name("texture")
                    .short("t")
                    .long("texture")
                    .help("Add a texture")
                    .takes_value(true)
                    .multiple(true),
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
                    .help("Specify font for FPS counter")
                    .takes_value(true),
                Arg::with_name("renderer")
                    .long("renderer")
                    .help("Specify renderer to use")
                    .takes_value(true),
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

    fn from_args(args: &ArgMatches) -> Result<Self> {
        let mut textures = HashMap::new();
        if let Some(values) = args.values_of("textures") {
            for path in values {
                textures.insert(
                    path.to_string(),
                    TextureConfig {
                        path: path.to_string(),
                    },
                );
            }
        };

        let mut buffers = HashMap::new();
        buffers.insert(
            "__default__".to_string(),
            BufferConfig {
                vertex: match args.value_of("vertex") {
                    Some(value) => value.to_string(),
                    None => bail!("Must specify vertex shader"),
                },
                fragment: match args.value_of("fragment") {
                    Some(value) => value.to_string(),
                    None => bail!("Must specify fragment shader"),
                },
                textures: match args.values_of("textures") {
                    Some(values) => values.map(|value| value.to_string()).collect(),
                    None => buffer_config::default_textures(),
                },
                width: match args.value_of("width") {
                    Some(value) => value.parse::<u32>()?,
                    None => buffer_config::default_width(),
                },
                height: match args.value_of("height") {
                    Some(value) => value.parse::<u32>()?,
                    None => buffer_config::default_height(),
                },
                resizeable: !(args.is_present("width") || args.is_present("height")),
                depends: buffer_config::default_depends(),
            },
        );

        Ok(Self {
            buffers: buffers,
            textures: textures,
            maximize: args.is_present("maximize"),
            vsync: args.is_present("vsync"),
            fps: args.is_present("fps"),
            font: match args.value_of("font") {
                Some(value) => value.to_string(),
                None => default_font(),
            },
            renderer: match args.value_of("renderer") {
                Some(value) => value.to_string(),
                None => default_renderer(),
            },
            platform_config: PlatformSpecificConfig::from_args(args),
        })
    }

    fn from_file(path: &Path) -> Result<Self> {
        info!("Using config file: {:?}", path);
        let file = File::open(path).chain_err(|| "Unable to open config file")?;
        let mut reader = BufReader::new(file);
        let mut config_str = String::new();
        reader
            .read_to_string(&mut config_str)
            .chain_err(|| "Could not read config file")?;
        ::std::env::set_current_dir(Path::new(path).parent().unwrap()).chain_err(|| "Failed to set current directory")?;
        Ok(::serde_yaml::from_str(&config_str)?)
    }

    pub fn parse() -> Result<Self> {
        let app = PlatformSpecificConfig::build_cli();
        let args = app.get_matches();

        match args.value_of("config") {
            Some(path) => Self::from_file(Path::new(path)),
            None => Self::from_args(&args),
        }
    }
}
