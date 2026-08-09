use std::{net::SocketAddr, num::NonZeroUsize, time::Duration};

use serde::Deserialize;
use url::Url;

/// Describes how the API derives the connecting client address.
#[derive(Debug, Clone, Deserialize)]
pub enum ProxyMode {
    /// Accepts connections without a trusted proxy.
    Direct,
    /// Trusts a fixed number of proxy hops.
    Trusted {
        /// Number of trusted proxy hops.
        hops: NonZeroUsize,
    },
}

/// Configures the HTTP API server and middleware.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiConfig {
    /// Local socket on which the API listens.
    pub bind_addr: SocketAddr,
    /// Public URL used to address this API.
    pub public_base_url: url::Url,
    /// Maximum duration of a request.
    pub request_timeout: std::time::Duration,
    /// Time allowed for graceful shutdown.
    pub graceful_shutdown_timeout: std::time::Duration,
    /// Maximum accepted request-body size in bytes.
    pub max_request_body_size: usize,
    /// Browser origins trusted by CORS and authentication.
    pub cors_allowed_origins: Vec<url::Url>,
    /// Optional parent domain that lets the web app read the CSRF cookie.
    pub csrf_cookie_domain: Option<String>,
    /// Client-address proxy policy.
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
            max_request_body_size: 1024 * 1024, // 1 MiB
            cors_allowed_origins: vec![
                Url::parse("http://localhost:5173").expect("Failed to build default config"),
            ],
            csrf_cookie_domain: None,
            proxy_mode: ProxyMode::Direct,
        }
    }
}
