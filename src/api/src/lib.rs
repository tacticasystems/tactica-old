use axum::{Router, routing::get};

pub use axum::serve;
use tower_http::request_id::MakeRequestUuid;

use crate::config::ApiConfig;

pub mod config;

pub fn router(_cfg: ApiConfig) -> Router {
    Router::new()
        .route("/healthz", get(|| async { "OK" }))

        .layer(tower_http::request_id::PropagateRequestIdLayer::x_request_id())
        .layer(
            tower_http::trace::TraceLayer::new_for_http()
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
        .layer(tower_http::request_id::SetRequestIdLayer::x_request_id(MakeRequestUuid::default()))
}
