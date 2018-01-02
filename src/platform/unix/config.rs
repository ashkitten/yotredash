//! Contains extra Unix-specific configurations

use clap::{App, Arg, ArgMatches};

use Config;

/// Platform-specific configuration
#[derive(Deserialize, Default, Clone)]
pub struct PlatformSpecificConfig {
    // TODO: implement
    /// Whether or not to draw on the root window
    #[serde(default = "default_root")]
    pub root: bool,

    /// Whether or not to use an override-redirect window. This allows the window to be controlled
    /// independently of the window manager, so you can use it as a desktop wallpaper or such
    #[serde(default = "default_override_redirect")]
    pub override_redirect: bool,

    /// Whether or not to set _NET_WM_WINDOW_TYPE_DESKTOP, allowing the window to be displayed as a
    /// desktop wallpaper in environments like GNOME
    #[serde(default = "default_desktop")]
    pub desktop: bool,

    /// Whether or not to lower window. This allows the window to be sent to the back; useful
    /// alongside the override_redirect option
    #[serde(default = "default_lower_window")]
    pub lower_window: bool,
}

/// A function that returns the default value of the `root` field
fn default_root() -> bool {
    false
}

/// A function that returns the default value of the `override_redirect` field
fn default_override_redirect() -> bool {
    false
}

/// A function that returns the default value of the `desktop` field
fn default_desktop() -> bool {
    false
}

/// A function that returns the default value of the `lower_window` field
fn default_lower_window() -> bool {
    false
}

impl PlatformSpecificConfig {
    /// Builds the application description needed to parse command-line arguments
    pub fn build_cli() -> App<'static, 'static> {
        Config::build_cli().args(&[
            Arg::with_name("root")
                .long("root")
                .help("Display on the root window"),
            Arg::with_name("override_redirect")
                .long("override-redirect")
                .help("Display as an override-redirect window"),
            Arg::with_name("desktop")
                .long("desktop")
                .help("Display as a desktop window"),
            Arg::with_name("lower_window")
                .long("lower-window")
                .help("Lower window to the bottom of the stack"),
        ])
    }

    /// Parses the configuration from command-line arguments
    pub fn from_args(args: &ArgMatches) -> Self {
        Self {
            root: args.is_present("root"),
            override_redirect: args.is_present("override_redirect"),
            desktop: args.is_present("desktop"),
            lower_window: args.is_present("lower_window"),
        }
    }
}
