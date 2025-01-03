use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;
use directories::ProjectDirs;

#[allow(dead_code)]
#[derive(Debug, Deserialize, Default)]
pub struct Config {
    pub database: DBConfig,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Default)]
pub struct DBConfig {
    #[serde(default)]
    pub db_type: String,
    pub url: Option<String>,
    pub filepath: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        if let Some(config_path) = Self::config_path() {
            if config_path.exists() {
                let config_str = std::fs::read_to_string(&config_path)
                    .context("Failed to read config file")?;
                
                let config: Config = serde_yaml::from_str(&config_str)
                    .context("Failed to parse config file")?;
                
                return Ok(config);
            }
        }

        Ok(Config {
            database: DBConfig {
                db_type: "sqlite".to_string(),
                url: None,
                filepath: None,
            },
        })
    }

    fn config_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "kweeb-logger", "logger")
            .map(|proj_dirs| proj_dirs.config_dir().join("config.yaml"))
    }
}