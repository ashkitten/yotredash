extern crate clap;

use clap::ArgMatches;

#[derive(Deserialize, Default)]
pub struct PlatformSpecificConfig {
    #[serde(default = "default_root")]
    pub root: bool,
    #[serde(default = "default_override_redirect")]
    pub override_redirect: bool,
    #[serde(default = "default_desktop")]
    pub desktop: bool,
    #[serde(default = "default_lower_window")]
    pub lower_window: bool,
}

fn default_root() -> bool {
    false
}

fn default_override_redirect() -> bool {
    false
}

fn default_desktop() -> bool {
    false
}

fn default_lower_window() -> bool {
    false
}

impl PlatformSpecificConfig {
    pub fn from_args(args: &ArgMatches) -> Self {
        Self {
            root: args.is_present("root"),
            override_redirect: args.is_present("override-redirect"),
            desktop: args.is_present("desktop"),
            lower_window: args.is_present("lower-window"),
        }
    }
}
