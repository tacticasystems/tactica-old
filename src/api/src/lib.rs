use axum::{Router, routing::get};

pub use axum::serve;

pub fn router() -> Router {
    Router::new()
        .route("/healthz", get(|| async { "OK" }))
}
