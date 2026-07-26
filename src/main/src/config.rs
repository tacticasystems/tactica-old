use figment::{Figment, providers::Env};
use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Deserialize)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,

    #[serde(default)]
    pub telemetry: TelemetryConfig,
}

impl Config {
    pub fn load() -> Result<Self, figment::Error> {
        Figment::new()
            .merge(Env::prefixed("TACTICA_").split("__"))
            .extract()
    }
}

#[derive(Deserialize)]
pub struct ServerConfig {
    #[serde(default = "ServerConfig::default_listen_addr")]
    pub listen_addr: SocketAddr,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            listen_addr: ServerConfig::default_listen_addr(),
        }
    }
}

impl ServerConfig {
    fn default_listen_addr() -> SocketAddr {
        ([0, 0, 0, 0], 8080).into()
    }
}

#[derive(Deserialize)]
pub struct TelemetryConfig {
    #[serde(default = "TelemetryConfig::default_env_filter")]
    pub log_level: String,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        TelemetryConfig {
            log_level: TelemetryConfig::default_env_filter(),
        }
    }
}

impl TelemetryConfig {
    fn default_env_filter() -> String {
        "info".into()
    }
}
