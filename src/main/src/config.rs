use figment::{Figment, providers::Env};
use serde::Deserialize;
use tactica_api::config::ApiConfig;

#[derive(Deserialize)]
pub struct Config {
    #[serde(default)]
    pub api: ApiConfig,

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
