use std::{net::SocketAddr, num::NonZeroUsize, time::Duration};

use serde::{Deserialize, Deserializer, de};
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
    #[serde(default, deserialize_with = "deserialize_csrf_cookie_domain")]
    pub csrf_cookie_domain: Option<String>,
    /// Client-address proxy policy.
    pub proxy_mode: ProxyMode,
}

fn deserialize_csrf_cookie_domain<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)?
        .map(validate_csrf_cookie_domain)
        .transpose()
        .map_err(de::Error::custom)
}

fn validate_csrf_cookie_domain(value: String) -> Result<String, &'static str> {
    if value.is_empty() || value.trim() != value || value.ends_with('.') {
        return Err("CSRF cookie domain must be a non-empty domain name");
    }

    let domain = value.strip_prefix('.').unwrap_or(&value);
    if domain.is_empty()
        || domain.split('.').any(|label| {
            label.is_empty()
                || label.starts_with('-')
                || label.ends_with('-')
                || !label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
    {
        return Err("CSRF cookie domain must contain only valid domain labels");
    }

    Ok(value.to_ascii_lowercase())
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

#[cfg(test)]
mod tests {
    use super::validate_csrf_cookie_domain;

    #[test]
    fn csrf_cookie_domain_is_normalized() {
        assert_eq!(
            validate_csrf_cookie_domain(".TACTICA.SYSTEMS".into()).unwrap(),
            ".tactica.systems"
        );
    }

    #[test]
    fn csrf_cookie_domain_rejects_cookie_attributes() {
        assert!(validate_csrf_cookie_domain(".tactica.systems; Secure".into()).is_err());
        assert!(validate_csrf_cookie_domain("https://tactica.systems".into()).is_err());
    }
}
