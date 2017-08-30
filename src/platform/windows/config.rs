extern crate clap;

use clap::ArgMatches;

#[derive(Deserialize, Default)]
pub struct PlatformSpecificConfig {}

impl PlatformSpecificConfig {
    pub fn from_args(args: &ArgMatches) -> Self {
        Self {}
    }

    pub fn build_cli() -> App<'static, 'static> {
        Config::build_cli()
    }
}
