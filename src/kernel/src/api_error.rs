use std::error::Error;

use http::StatusCode;

/// An error that can be rendered as a stable HTTP API response.
pub trait ApiError: Error + Send + Sync + 'static {
    /// Returns the user-facing error message.
    fn message(&self) -> String;
    /// Returns the stable machine-readable error code.
    fn code(&self) -> &'static str;
    /// Returns the HTTP status associated with the error.
    fn status_code(&self) -> StatusCode;
}

#[cfg(feature = "axum")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
/// JSON body returned for an API error.
pub struct ApiErrorJson {
    /// User-facing error message.
    pub message: String,
    /// Stable machine-readable error code.
    pub code: &'static str,
}

/// Type-erased API error response.
pub struct ApiErrorResponse {
    error: Box<dyn ApiError>,
}

impl<E> From<E> for ApiErrorResponse
where
    E: ApiError,
{
    fn from(error: E) -> Self {
        Self {
            error: Box::new(error),
        }
    }
}

#[cfg(feature = "axum")]
impl axum::response::IntoResponse for ApiErrorResponse {
    fn into_response(self) -> axum::response::Response {
        let status = self.error.status_code();
        let body = ApiErrorJson {
            message: self.error.message(),
            code: self.error.code(),
        };

        (status, axum::Json(body)).into_response()
    }
}

/// Result type used by API handlers.
pub type ApiResult<T> = Result<T, ApiErrorResponse>;
