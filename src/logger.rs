use std::{fs, io, path::PathBuf};

use chrono::Local;
use fern::colors::{Color, ColoredLevelConfig};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LoggerConfig {
    pub console: ConsoleLoggerConfig,
    pub file: Option<FileLoggerConfig>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsoleLoggerConfig {
    #[serde(default = "default_console_log_level")]
    pub level: log::LevelFilter,
}

impl Default for ConsoleLoggerConfig {
    fn default() -> Self {
        Self {
            level: log::LevelFilter::Debug,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileLoggerConfig {
    #[serde(default = "default_file_log_level")]
    pub level: log::LevelFilter,
    #[serde(default = "default_file_log_path")]
    pub path: PathBuf,
}

impl Default for FileLoggerConfig {
    fn default() -> Self {
        Self {
            level: default_file_log_level(),
            path: default_file_log_path(),
        }
    }
}

fn default_console_log_level() -> log::LevelFilter {
    log::LevelFilter::Debug
}

fn default_file_log_level() -> log::LevelFilter {
    log::LevelFilter::Debug
}

fn default_file_log_path() -> PathBuf {
    let mut path = PathBuf::new();
    path.set_file_name("archive.log");
    path
}

impl LoggerConfig {
    pub fn init(self) -> io::Result<()> {
        let colors = ColoredLevelConfig::new()
            .info(Color::Green)
            .warn(Color::Yellow)
            .error(Color::Red)
            .debug(Color::Blue)
            .trace(Color::Magenta);

        let stdout_dispatcher = fern::Dispatch::new()
            .level(self.console.level)
            .level_for("archive", self.console.level)
            .level_for("sqlx", log::LevelFilter::Error)
            .level_for("frame_executive", log::LevelFilter::Error)
            .format(move |out, message, record| {
                out.finish(format_args!(
                    "{} [{}] {}: {}",
                    Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                    colors.color(record.level()),
                    record.target(),
                    message,
                ))
            })
            .chain(io::stdout());

        if let Some(file) = self.file {
            fs::create_dir_all(
                file.path
                    .parent()
                    .expect("Cannot get the parent path of log file"),
            )?;

            let file_dispatcher = fern::Dispatch::new()
                .level(file.level)
                .level_for("archive", file.level)
                .level_for("sqlx", log::LevelFilter::Error)
                .level_for("frame_executive", log::LevelFilter::Error)
                .format(move |out, message, record| {
                    out.finish(format_args!(
                        "{} [{}] [{};{}] {}: {}",
                        Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                        record.level(),
                        record.file().unwrap_or_default(),
                        record.line().map(|l| l.to_string()).unwrap_or_default(),
                        record.target(),
                        message,
                    ))
                })
                .chain(fern::log_file(file.path)?);

            fern::Dispatch::new()
                .chain(stdout_dispatcher)
                .chain(file_dispatcher)
                .apply()
                .expect("Could not init logging");
        } else {
            stdout_dispatcher.apply().expect("Could not init logging");
        }

        Ok(())
    }
}
