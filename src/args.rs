extern crate clap;
extern crate json;

// Clap

use clap::{App, Arg, ArgMatches};

// Std

use std;

// Functions

pub fn build_cli() -> App<'static, 'static> {

    let app = clap::App::new("yotredash")
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

pub fn parse_args<'a>() -> ArgMatches<'a> {
    build_cli().get_matches()
}

pub fn apply_args(args: &ArgMatches, config: &mut json::JsonValue) {
    config["output"]["vertex"] = args.value_of("vertex")
        .unwrap_or_else(|| {
            error!("Must specify vertex shader!");
            args.usage();
            std::process::exit(1);
        })
        .into();

    config["output"]["fragment"] = args.value_of("fragment")
        .unwrap_or_else(|| {
            error!("Must specify fragment shader!");
            args.usage();
            std::process::exit(1);
        })
        .into();

    if let Some(textures) = args.values_of("textures") {
        config["output"]["textures"] = json::JsonValue::Array(
            textures
                .map(|texture: &str| json::JsonValue::from(texture))
                .collect(),
        )
    }

    if let Some(width) = args.value_of("width") {
        config["output"]["width"] = width.parse::<i64>().unwrap().into();
    }

    if let Some(height) = args.value_of("height") {
        config["output"]["height"] = height.parse::<i64>().unwrap().into();
    }

    if args.is_present("maximize") {
        config["maximize"] = true.into();
    }

    if args.is_present("fullscreen") {
        config["fullscreen"] = true.into();
    }

    if args.is_present("vsync") {
        config["vsync"] = true.into();
    }

    if args.is_present("fps") {
        config["fps"] = true.into();
    }

    if let Some(font) = args.value_of("font") {
        config["font"] = font.into();
    }

    if args.is_present("root") {
        config["root"] = true.into();
    }

    if args.is_present("override-redirect") {
        config["override_redirect"] = true.into();
    }

    if args.is_present("desktop") {
        config["desktop"] = true.into();
    }

    if args.is_present("lower-window") {
        config["lower_window"] = true.into();
    }
}
