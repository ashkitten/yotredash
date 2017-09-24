use clap::{App, ArgMatches};

use config::Config;

/// Platform-specific configuration
#[derive(Deserialize, Default, Clone)]
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
