use std::fs;

use directories::ProjectDirs;
use serde::Deserialize;

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct Config {
    pub api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ConfigFile {
    api_key: Option<String>,
}

pub fn load_config() -> Result<Config, AppError> {
    let env_key = std::env::var("OPENROUTER_API_KEY").ok();
    let project_dirs = ProjectDirs::from("ai", "openrouter", "openrouter-tui");

    let file_key = if let Some(dirs) = project_dirs {
        let path = dirs.config_dir().join("config.toml");
        if path.exists() {
            let contents =
                fs::read_to_string(&path).map_err(|err| AppError::Config(err.to_string()))?;
            let parsed: ConfigFile =
                toml::from_str(&contents).map_err(|err| AppError::Config(err.to_string()))?;
            parsed.api_key
        } else {
            None
        }
    } else {
        None
    };

    Ok(Config {
        api_key: env_key.or(file_key),
    })
}
