use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};
use structopt::StructOpt;

use archive_actor::{KafkaConfig, PostgresConfig};
use archive_client::ClientConfig;

use crate::{error::ArchiveError, logger::LoggerConfig};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArchiveConfig {
    pub(crate) logger: LoggerConfig,
    pub(crate) client: ClientConfig,
    pub(crate) postgres: PostgresConfig,
    pub(crate) kafka: Option<KafkaConfig>,
}

#[derive(Clone, Debug, StructOpt)]
#[structopt(author, about)]
pub struct ArchiveCli {
    /// Specifies the archive config file.
    #[structopt(short, long, name = "FILE")]
    config: PathBuf,
}

impl ArchiveCli {
    pub fn init() -> Result<ArchiveConfig, ArchiveError> {
        let cli: Self = StructOpt::from_args();
        let toml_str = fs::read_to_string(cli.config.as_path())?;
        let config = toml::from_str::<ArchiveConfig>(toml_str.as_str())?;
        // initialize the logger
        config.logger.clone().init()?;
        Ok(config)
    }
}
