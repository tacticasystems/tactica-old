use figment::{Figment, providers::Env};
use serde::Deserialize;
use tactica_api::config::ApiConfig;
use tactica_db::config::DatabaseConfig;
use tactica_module_identity::service::AuthConfig;

#[derive(Deserialize)]
pub struct Config {
    #[serde(default)]
    pub api: ApiConfig,

    #[serde(default)]
    pub telemetry: TelemetryConfig,

    #[serde(default)]
    pub database: DatabaseConfig,

    #[serde(default)]
    pub auth: AuthConfig,
}

impl Config {
    pub fn load() -> Result<Self, Box<figment::Error>> {
        Figment::new()
            .merge(Env::prefixed("TACTICA_").split("__"))
            .extract()
            .map_err(Box::new)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use figment::Figment;

    use super::Config;

    #[test]
    fn api_environment_overrides_keep_other_api_defaults() {
        let config: Config = Figment::new()
            .merge(("api.bind_addr", "127.0.0.1:9090"))
            .extract()
            .expect("partial API configuration should load");

        assert_eq!(config.api.bind_addr.to_string(), "127.0.0.1:9090");
        assert_eq!(
            config.api.public_base_url.as_str(),
            "http://localhost:8080/"
        );
        assert_eq!(config.api.request_timeout, Duration::from_secs(30));
        assert_eq!(config.api.max_request_body_size, 1024 * 1024);
        assert!(matches!(
            config.api.proxy_mode,
            tactica_api::config::ProxyMode::Direct
        ));
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
