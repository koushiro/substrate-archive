use std::{fs, path::PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use structopt::StructOpt;

use crate::logger::LoggerConfig;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArchiveConfig {
    logger: LoggerConfig,
}

#[derive(Clone, Debug, StructOpt)]
#[structopt(author, about)]
pub struct ArchiveCli {
    /// Specifies the config file.
    #[structopt(short = "c", long, name = "FILE")]
    config: PathBuf,
}

impl ArchiveCli {
    pub fn init() -> Result<ArchiveConfig> {
        let cli: Self = ArchiveCli::from_args();
        let toml_str = fs::read_to_string(cli.config.as_path())?;
        let config = toml::from_str::<ArchiveConfig>(toml_str.as_str())?;
        // initialize the logger
        config.logger.clone().init()?;
        Ok(config)
    }
}
