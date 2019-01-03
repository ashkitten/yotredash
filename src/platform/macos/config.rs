use clap::{App, ArgMatches};
use serde_derive::Deserialize;

use config::Config;

/// Platform-specific configuration
/// Be careful with this, because specifying an unknown field will not cause an error
#[derive(Debug, Deserialize, Default, Clone)]
pub struct PlatformSpecificConfig {}

impl PlatformSpecificConfig {
    /// Builds the application description needed to parse command-line arguments
    pub fn build_cli() -> App<'static, 'static> {
        Config::build_cli()
    }

    /// Parses the configuration from command-line arguments
    pub fn from_args(args: &ArgMatches) -> Self {
        Self {}
    }
}
