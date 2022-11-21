use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use toml::Value;
use uuid::Uuid;

static DEFAULT_WEBSOCKET_HOST: Lazy<SocketAddr> =
    Lazy::new(|| SocketAddr::from(([0, 0, 0, 0], 8080)));

#[derive(Debug)]
pub struct ConfigError {
    message: String,
}

impl ConfigError {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message.as_str())
    }
}

impl Error for ConfigError {}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub jmri_host: SocketAddr,
    pub server_host: SocketAddr,

    #[serde(skip_serializing, skip_deserializing)]
    pub uuid: String,
}

impl Config {
    pub fn get() -> Result<Config, ConfigError> {
        if !Path::new("config.toml").exists() {
            return Err(ConfigError {
                message: "Unable to find 'config.toml'".to_string(),
            });
        }

        let file = fs::read_to_string("config.toml")
            .map_err(|_| ConfigError::new("Unable to find config file".to_string()))?;

        let values: Value = file
            .parse()
            .map_err(|e| ConfigError::new(format!("Error parsing config file: {}", e)))?;

        let jmri_host = values
            .get("jmri_host")
            .and_then(|h| h.as_str())
            .ok_or_else(|| ConfigError::new("Unable to find 'jmri_host' config value".to_string()))?
            .parse::<SocketAddr>()
            .map_err(|e| {
                ConfigError::new(format!("Error parsing 'jmri_host' config value: {}", e))
            })?;

        // .and_then(|h| h.parse::<SocketAddr>().ok())
        // .ok_or_else(|| ConfigError::new("Unable to read 'jmri_host' config value".to_string()))?;
        let server_host = values
            .get("server_host")
            .and_then(|host| host.as_str())
            .and_then(|host| host.parse::<SocketAddr>().ok())
            .unwrap_or(*DEFAULT_WEBSOCKET_HOST);

        Ok(Config {
            jmri_host,
            server_host,
            uuid: Uuid::new_v4().to_string(),
        })
    }
}
