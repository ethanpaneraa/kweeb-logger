use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;
use directories::ProjectDirs;
use std::env;

#[allow(dead_code)]
#[derive(Debug, Deserialize, Default)]
pub struct Config {
    pub database: DBConfig,
    #[serde(default)]
    pub supabase: SupabaseConfig,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Default)]
pub struct DBConfig {
    #[serde(default)]
    pub db_type: String,
    pub url: Option<String>,
    pub filepath: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct SupabaseConfig {
    pub enabled: bool,
    pub url: Option<String>,
    pub api_key: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        // Try to load from file first
        let mut config = if let Some(config_path) = Self::config_path() {
            if config_path.exists() {
                let config_str = std::fs::read_to_string(&config_path)
                    .context("Failed to read config file")?;
                
                let config: Config = serde_yaml::from_str(&config_str)
                    .context("Failed to parse config file")?;
                
                config
        } else {
                Config::default()
            }
        } else {
            Config::default()
        };

        // Check environment variables and override config if they exist
        if let Ok(url) = env::var("SUPABASE_URL") {
            config.supabase.url = Some(url);
            config.supabase.enabled = true;
        }

        if let Ok(api_key) = env::var("SUPABASE_ANON_KEY") {
            config.supabase.api_key = Some(api_key);
            config.supabase.enabled = true;
        }

        log::debug!("Loaded config: {:?}", config);
        Ok(config)
    }

    fn config_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "kweeb-logger", "logger")
            .map(|proj_dirs| proj_dirs.config_dir().join("config.yaml"))
    }

    pub fn has_supabase_config(&self) -> bool {
        self.supabase.enabled && 
        self.supabase.url.is_some() && 
        self.supabase.api_key.is_some()
    }
}