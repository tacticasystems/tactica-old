use std::{net::SocketAddr, num::NonZeroUsize, time::Duration};

use serde::Deserialize;
use url::Url;

#[derive(Debug, Clone, Deserialize)]
pub enum ProxyMode {
    Direct,
    Trusted { hops: NonZeroUsize },
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiConfig {
    pub bind_addr: SocketAddr,
    pub public_base_url: url::Url,
    pub request_timeout: std::time::Duration,
    pub graceful_shutdown_timeout: std::time::Duration,
    pub max_request_body_size: usize,
    pub cors_allowed_origins: Vec<url::Url>,
    pub proxy_mode: ProxyMode,
}

impl ApiConfig {
    fn default_listen_addr() -> SocketAddr {
        ([0, 0, 0, 0], 8080).into()
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            bind_addr: Self::default_listen_addr(),
            public_base_url: Url::parse("http://localhost:8080")
                .expect("Failed to build default config"),
            request_timeout: Duration::from_secs(30),
            graceful_shutdown_timeout: Duration::from_secs(10),
            max_request_body_size: 1 * 1024 * 1024, // 1 MiB
            cors_allowed_origins: vec![
                Url::parse("http://localhost:5173").expect("Failed to build default config"),
            ],
            proxy_mode: ProxyMode::Direct,
        }
    }
}
