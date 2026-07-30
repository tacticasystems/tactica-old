use axum::{Router, routing::get};

pub use axum::serve;
use http::StatusCode;
use tower::{ServiceBuilder};
use tower_http::{cors::{AllowOrigin, CorsLayer}, limit::RequestBodyLimitLayer, request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer}, timeout::TimeoutLayer, trace::TraceLayer};

use crate::config::ApiConfig;

pub mod config;

pub fn router(cfg: ApiConfig) -> Router {
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
                .on_response(|response: &axum::http::Response<_>, latency: std::time::Duration, _span: &tracing::Span| {
                    tracing::info!(
                        status = %response.status(),
                        latency_ms = latency.as_millis(),
                        "response sent",
                    );
                })
        )
        .layer(
            CorsLayer::new()
                .allow_origin(
                    AllowOrigin::list(
                        cfg
                            .cors_allowed_origins
                            .iter()
                            .map(|e| e.as_str().parse().expect("Invalid CORS origin"))
                    )
                )
                .allow_credentials(true)
                .allow_methods(vec![
                    http::Method::GET,
                    http::Method::PUT,
                    http::Method::POST,
                    http::Method::DELETE,
                    http::Method::HEAD,
                ])
        )
        .layer(RequestBodyLimitLayer::new(cfg.max_request_body_size))
        .layer(TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, cfg.request_timeout))
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid::default()));

    Router::new()
        .route("/healthz", get(|| async { "OK" }))
        .layer(middleware)
}
