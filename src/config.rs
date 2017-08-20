extern crate clap;
extern crate serde_yaml;

use clap::{App, Arg, ArgMatches};
use std;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use platform::config::PlatformSpecificConfig;

#[derive(Deserialize, Clone)]
pub struct BufferConfig {
    #[serde(default = "error_no_vertex")]
    pub vertex: String,
    #[serde(default = "error_no_fragment")]
    pub fragment: String,
    #[serde(default = "default_textures")]
    pub textures: Vec<String>,
    #[serde(default = "default_width")]
    pub width: u32,
    #[serde(default = "default_height")]
    pub height: u32,
    #[serde(default = "default_depends")]
    pub depends: Vec<String>,
}

fn error_no_vertex() -> String {
    error!("Must specify vertex shader");
    std::process::exit(1);
}

fn error_no_fragment() -> String {
    error!("Must specify fragment shader");
    std::process::exit(1);
}

fn default_textures() -> Vec<String> {
    Vec::new()
}

fn default_width() -> u32 {
    640
}

fn default_height() -> u32 {
    400
}

fn default_depends() -> Vec<String> {
    Vec::new()
}

#[derive(Deserialize)]
pub struct Config {
    pub buffers: BTreeMap<String, BufferConfig>,
    #[serde(default = "default_maximize")]
    pub maximize: bool,
    #[serde(default = "default_vsync")]
    pub vsync: bool,
    #[serde(default = "default_fps")]
    pub fps: bool,
    #[serde(default = "default_font")]
    pub font: String,
    #[serde(default)]
    pub platform_config: PlatformSpecificConfig,
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

impl Config {
    fn build_cli() -> App<'static, 'static> {
        let app = App::new("yotredash")
            .version("0.1.0")
            .author("Ash Levy <ashlea@protonmail.com>")
            .args(
                &[
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
                    Arg::with_name("maximize").long("maximize").help(
                        "Maximize window dimensions",
                    ),
                    Arg::with_name("fullscreen").long("fullscreen").help(
                        "Make window fullscreen",
                    ),
                    Arg::with_name("vsync").long("vsync").help(
                        "Enable vertical sync",
                    ),
                    Arg::with_name("fps").long("fps").help(
                        "Enable FPS log to console",
                    ),
                    Arg::with_name("font")
                        .long("font")
                        .help("Specify font for FPS counter")
                        .takes_value(true),
                    Arg::with_name("config")
                        .short("c")
                        .long("config")
                        .help("Load a config file")
                        .takes_value(true),
                ],
            )
            .after_help(
                "\
                 This program uses `env_logger` as its logging backend.\n\
                 Common usage: `RUST_LOG=yotredash=info yotredash`\n\
                 See http://rust-lang-nursery.github.io/log/env_logger/ for more information.\
                 ",
            );

        if cfg!(windows) {
            app
        } else if cfg!(unix) {
            (app) // TODO: remove parens, this is to trick rustfmt into formatting correctly
                .args(&[
                    Arg::with_name("root")
                        .long("root")
                        .help("Display on the root window"),
                    Arg::with_name("override-redirect")
                        .long("override-redirect")
                        .help("Display as an override-redirect window"),
                    Arg::with_name("desktop")
                        .long("desktop")
                        .help("Display as a desktop window"),
                    Arg::with_name("lower-window")
                        .long("lower-window")
                        .help("Lower window to the bottom of the stack"),
                ])
        } else if cfg!(macos) {
            app
        } else {
            app
        }
    }

    fn from_args(args: &ArgMatches) -> Self {
        let mut buffers = BTreeMap::new();
        buffers.insert(
            "__default__".to_string(),
            BufferConfig {
                vertex: match args.value_of("vertex") {
                    Some(vertex) => vertex.to_string(),
                    None => error_no_vertex(),
                },
                fragment: match args.value_of("fragment") {
                    Some(fragment) => fragment.to_string(),
                    None => error_no_fragment(),
                },
                textures: match args.values_of("textures") {
                    Some(textures) => textures.map(|texture: &str| texture.to_string()).collect(),
                    None => default_textures(),
                },
                width: match args.value_of("width") {
                    Some(width) => width.parse::<u32>().unwrap(),
                    None => default_width(),
                },
                height: match args.value_of("height") {
                    Some(height) => height.parse::<u32>().unwrap(),
                    None => default_height(),
                },
                depends: default_depends(),
            },
        );

        Self {
            buffers: buffers,
            maximize: args.is_present("maximize"),
            vsync: args.is_present("vsync"),
            fps: args.is_present("fps"),
            font: match args.value_of("font") {
                Some(font) => font.to_string(),
                None => default_font(),
            },
            platform_config: PlatformSpecificConfig::from_args(args),
        }
    }

    pub fn parse() -> Self {
        let app = Self::build_cli();
        let args = app.get_matches();

        match args.value_of("config") {
            Some(path) => {
                let file = match File::open(path) {
                    Ok(file) => file,
                    Err(error) => {
                        error!("Could not open config file: {}", error);
                        std::process::exit(1);
                    }
                };
                let mut reader = BufReader::new(file);
                let mut config_str = String::new();
                match reader.read_to_string(&mut config_str) {
                    Ok(_) => info!("Using config file: {}", path),
                    Err(error) => {
                        error!("Could not read config file: {}", error);
                        std::process::exit(1);
                    }
                };
                std::env::set_current_dir(Path::new(path).parent().unwrap());
                serde_yaml::from_str(&config_str).unwrap()
            }
            None => Config::from_args(&args),
        }
    }
}
