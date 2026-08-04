use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DatabaseConfig {
    #[serde(default = "DatabaseConfig::default_url")]
    pub url: String,
    #[serde(default = "DatabaseConfig::default_max_connections")]
    pub max_connections: u32,
    #[serde(default = "DatabaseConfig::default_min_connections")]
    pub min_connections: u32,
    #[serde(default = "DatabaseConfig::default_connect_timeout")]
    pub connect_timeout: Duration,
    #[serde(default = "DatabaseConfig::default_idle_timeout")]
    pub idle_timeout: Duration,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        DatabaseConfig {
            url: Self::default_url(),
            max_connections: Self::default_max_connections(),
            min_connections: Self::default_min_connections(),
            connect_timeout: Self::default_connect_timeout(),
            idle_timeout: Self::default_idle_timeout(),
        }
    }
}

impl DatabaseConfig {
    fn default_url() -> String {
        "postgres://user:password@localhost:5432/mydb".to_string()
    }

    fn default_max_connections() -> u32 {
        10
    }

    fn default_min_connections() -> u32 {
        1
    }

    fn default_connect_timeout() -> Duration {
        Duration::from_secs(5)
    }

    fn default_idle_timeout() -> Duration {
        Duration::from_secs(300)
    }
}
