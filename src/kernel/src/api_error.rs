use std::error::Error;

use http::StatusCode;

pub trait ApiError: Error + Send + Sync + 'static {
    fn message(&self) -> String;
    fn code(&self) -> &'static str;
    fn status_code(&self) -> StatusCode;
}

#[cfg(feature = "axum")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ApiErrorJson {
    pub message: String,
    pub code: &'static str,
}

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

pub type ApiResult<T> = Result<T, ApiErrorResponse>;
