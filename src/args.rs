extern crate clap;

// Clap

use clap::{Arg, ArgMatches};

pub fn parse_args<'a>() -> ArgMatches<'a> {
    let app = clap::App::new("yotredash")
        .version("0.1.0")
        .author("Ash Levy <ashlea@protonmail.com>")
        .arg(
            Arg::with_name("vertex")
                .short("v")
                .long("vertex")
                .help("Specify a vertex shader")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("fragment")
                .short("f")
                .long("fragment")
                .help("Specify a fragment shader")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("channel")
                .short("c")
                .long("channel")
                .help("Add a channel")
                .takes_value(true)
                .multiple(true),
        )
        .arg(
            Arg::with_name("width")
                .short("w")
                .long("width")
                .help("Set window width")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("height")
                .short("h")
                .long("height")
                .help("Set window height")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("maximize")
                .long("maximize")
                .help("Maximize window dimensions"),
        )
        .arg(
            Arg::with_name("fullscreen")
                .long("fullscreen")
                .help("Make window fullscreen"),
        )
        .arg(
            Arg::with_name("vsync")
                .long("vsync")
                .help("Enable vertical sync"),
        )
        .arg(Arg::with_name("fps").long("fps").help("Enable FPS counter"))
        .arg(
            Arg::with_name("font")
                .long("font")
                .help("Specify font for FPS counter")
                .takes_value(true),
        )
        .after_help(
            "This program uses `env_logger` as its logging backend.\n\
             See http://rust-lang-nursery.github.io/log/env_logger/ for more information.",
        );

    let app = if cfg!(windows) {
        app
    } else if cfg!(unix) {
        (app) // TODO: remove parens, this is to trick rustfmt into formatting correctly
            .arg(
                Arg::with_name("root")
                    .long("root")
                    .help("Display on the root window"),
            )
            .arg(
                Arg::with_name("override-redirect")
                    .long("override-redirect")
                    .help("Display as an override-redirect window"),
            )
            .arg(
                Arg::with_name("desktop")
                    .long("desktop")
                    .help("Display as a desktop window"),
            )
            .arg(
                Arg::with_name("lower-window")
                    .long("lower-window")
                    .help("Lower window to the bottom of the stack"),
            )
    } else if cfg!(macos) {
        app
    } else {
        app
    };

    return app.get_matches();
}
