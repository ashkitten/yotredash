extern crate clap;

use clap::ArgMatches;

#[derive(Deserialize, Default)]
pub struct PlatformSpecificConfig {}

impl PlatformSpecificConfig {
    pub fn from_args(args: &ArgMatches) -> Self {
        Self {}
    }
}
