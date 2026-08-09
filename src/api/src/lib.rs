//! HTTP API composition and authentication handlers.
#![deny(missing_docs)]

use axum::{Router, routing::get};

/// Serves an Axum router on an asynchronous listener.
pub use axum::serve;
use http::StatusCode;
use tower::ServiceBuilder;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    limit::RequestBodyLimitLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

use crate::config::ApiConfig;

/// API configuration.
pub mod config;
/// HTTP route handlers.
pub mod handlers;
/// Shared application state.
pub mod state;

/// State shared by API handlers.
pub use state::AppState;

/// Builds the complete HTTP router.
pub fn router(cfg: ApiConfig, state: AppState) -> Router {
    let state = state
        .with_trusted_origins(
            cfg.cors_allowed_origins
                .iter()
                .map(|url| url.origin().ascii_serialization())
                .collect(),
        )
        .with_csrf_cookie_domain(cfg.csrf_cookie_domain.clone());
    let middleware = ServiceBuilder::new()
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &axum::http::Request<_>| {
                    let request_id = request
                        .headers()
                        .get("x-request-id")
                        .and_then(|value| value.to_str().ok())
                        .unwrap_or("<missing>");

                    tracing::info_span!(
                        "request",
                        method = %request.method(),
                        uri = %request.uri(),
                        request_id = %request_id,
                    )
                })
                .on_request(|_request: &axum::http::Request<_>, _span: &tracing::Span| {
                    tracing::info!("request received");
                })
                .on_response(
                    |response: &axum::http::Response<_>,
                     latency: std::time::Duration,
                     _span: &tracing::Span| {
                        tracing::info!(
                            status = %response.status(),
                            latency_ms = latency.as_millis(),
                            "response sent",
                        );
                    },
                ),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(
                    cfg.cors_allowed_origins
                        .iter()
                        .map(|e| e.as_str().parse().expect("Invalid CORS origin")),
                ))
                .allow_credentials(true)
                .allow_methods(vec![
                    http::Method::GET,
                    http::Method::PUT,
                    http::Method::POST,
                    http::Method::DELETE,
                    http::Method::HEAD,
                ]),
        )
        .layer(RequestBodyLimitLayer::new(cfg.max_request_body_size))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            cfg.request_timeout,
        ))
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid));

    Router::new()
        .route("/healthz", get(|| async { "OK" }))
        .nest("/api/v1", api_router())
        .layer(middleware)
        .with_state(state)
}

fn api_router() -> Router<AppState> {
    Router::new().merge(handlers::identity::router())
}
