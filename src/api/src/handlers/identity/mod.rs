use axum::{
    Json, Router,
    extract::State,
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{AUTHORIZATION, ORIGIN, SET_COOKIE},
    },
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tactica_kernel::api_error::{ApiError, ApiResult};
use tactica_module_identity::{
    models::{
        account::EmailAddress,
        session::{CurrentSession, IssuedSession},
    },
    ports::AuthError,
};
use thiserror::Error;

use crate::AppState;

const SESSION_COOKIE: &str = "tactica_session";
const CSRF_COOKIE: &str = "tactica_csrf";

/// Builds the versioned authentication router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/session", get(session))
        .route("/auth/verify-email", post(verify_email))
        .route("/auth/resend-verification", post(resend_verification))
}

#[derive(Debug, Error)]
enum HttpAuthError {
    #[error("cookie and bearer authentication are mutually exclusive")]
    AmbiguousCredentials,
    #[error("the request origin is not trusted")]
    UntrustedOrigin,
    #[error("a valid X-CSRF-Token header and matching cookie are required")]
    InvalidCsrf,
    #[error("this route requires an anonymous client")]
    AlreadyAuthenticated,
}

impl ApiError for HttpAuthError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::AmbiguousCredentials => StatusCode::BAD_REQUEST,
            Self::UntrustedOrigin | Self::InvalidCsrf | Self::AlreadyAuthenticated => {
                StatusCode::FORBIDDEN
            }
        }
    }
    fn message(&self) -> String {
        self.to_string()
    }
    fn code(&self) -> &'static str {
        match self {
            Self::AmbiguousCredentials => "ambiguous_credentials",
            Self::UntrustedOrigin => "untrusted_origin",
            Self::InvalidCsrf => "invalid_csrf_token",
            Self::AlreadyAuthenticated => "already_authenticated",
        }
    }
}

#[derive(Deserialize)]
struct CredentialsBody {
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct VerificationBody {
    code: String,
}

#[derive(Serialize)]
struct AuthResponse {
    account_id: String,
    email_verified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_token: Option<String>,
}

#[derive(Serialize)]
struct SessionResponse {
    account_id: String,
    email_verified: bool,
    session: SessionTimes,
}

#[derive(Serialize)]
struct SessionTimes {
    created_at: DateTime<Utc>,
    idle_expires_at: DateTime<Utc>,
    absolute_expires_at: DateTime<Utc>,
}

enum Transport<'a> {
    Cookie(&'a str),
    Bearer(&'a str),
}

fn cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(axum::http::header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|part| {
            let (key, value) = part.trim().split_once('=')?;
            (key == name).then_some(value)
        })
}

fn credentials(headers: &HeaderMap) -> Result<Option<Transport<'_>>, HttpAuthError> {
    let cookie = cookie(headers, SESSION_COOKIE);
    let bearer = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    match (cookie, bearer) {
        (Some(_), Some(_)) => Err(HttpAuthError::AmbiguousCredentials),
        (Some(value), None) => Ok(Some(Transport::Cookie(value))),
        (None, Some(value)) => Ok(Some(Transport::Bearer(value))),
        (None, None) => Ok(None),
    }
}

fn trusted_origin<'a>(
    state: &'a AppState,
    headers: &'a HeaderMap,
) -> Result<Option<&'a str>, HttpAuthError> {
    let Some(value) = headers.get(ORIGIN) else {
        return Ok(None);
    };
    let origin = value.to_str().map_err(|_| HttpAuthError::UntrustedOrigin)?;
    if state
        .trusted_origins
        .iter()
        .any(|trusted| trusted == origin)
    {
        Ok(Some(origin))
    } else {
        Err(HttpAuthError::UntrustedOrigin)
    }
}

/// Guard for routes that require no valid Session.
pub async fn anonymous(state: &AppState, headers: &HeaderMap) -> ApiResult<()> {
    let Some(transport) = credentials(headers)? else {
        return Ok(());
    };
    let token = match transport {
        Transport::Cookie(v) | Transport::Bearer(v) => v,
    };
    match state.identity_service.authenticate(token).await {
        Ok(_) => Err(HttpAuthError::AlreadyAuthenticated.into()),
        Err(AuthError::Unauthorized) => Ok(()),
        Err(error) => Err(error.into()),
    }
}

/// Guard for routes that require a valid Session, including an unverified Account.
pub async fn authenticated(
    state: &AppState,
    headers: &HeaderMap,
    unsafe_method: bool,
) -> ApiResult<CurrentSession> {
    let transport = credentials(headers)?.ok_or(AuthError::Unauthorized)?;
    let (token, cookie_auth) = match transport {
        Transport::Cookie(v) => (v, true),
        Transport::Bearer(v) => (v, false),
    };
    let current = state.identity_service.authenticate(token).await?;
    if cookie_auth && unsafe_method {
        trusted_origin(state, headers)?;
        let header = headers
            .get("x-csrf-token")
            .and_then(|v| v.to_str().ok())
            .ok_or(HttpAuthError::InvalidCsrf)?;
        let csrf_cookie = cookie(headers, CSRF_COOKIE).ok_or(HttpAuthError::InvalidCsrf)?;
        if header != csrf_cookie {
            return Err(HttpAuthError::InvalidCsrf.into());
        }
        state
            .identity_service
            .validate_csrf(&current, header)
            .await
            .map_err(|_| HttpAuthError::InvalidCsrf)?;
    }
    Ok(current)
}

/// Guard for routes that require a verified Account.
pub async fn verified(
    state: &AppState,
    headers: &HeaderMap,
    unsafe_method: bool,
) -> ApiResult<CurrentSession> {
    let current = authenticated(state, headers, unsafe_method).await?;
    if !current.email_verified {
        return Err(AuthError::EmailVerificationRequired.into());
    }
    Ok(current)
}

async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CredentialsBody>,
) -> ApiResult<Response> {
    anonymous(&state, &headers).await?;
    let cookie_mode = trusted_origin(&state, &headers)?.is_some();
    let email = EmailAddress::new(&body.email)?;
    let issued = state
        .identity_service
        .register(email, &body.password)
        .await?;
    Ok(issue_response(
        issued,
        cookie_mode,
        state.csrf_cookie_domain.as_deref(),
    ))
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CredentialsBody>,
) -> ApiResult<Response> {
    anonymous(&state, &headers).await?;
    let cookie_mode = trusted_origin(&state, &headers)?.is_some();
    let email = EmailAddress::new(&body.email)?;
    let issued = state.identity_service.login(email, &body.password).await?;
    Ok(issue_response(
        issued,
        cookie_mode,
        state.csrf_cookie_domain.as_deref(),
    ))
}

fn csrf_cookie(value: &str, domain: Option<&str>, expired: bool) -> String {
    csrf_cookie_with_path(value, "/", domain, expired)
}

fn csrf_cookie_with_path(value: &str, path: &str, domain: Option<&str>, expired: bool) -> String {
    let domain = domain
        .map(|value| format!("; Domain={value}"))
        .unwrap_or_default();
    let expiry = expired.then_some("; Max-Age=0").unwrap_or_default();
    format!("{CSRF_COOKIE}={value}; Path={path}{expiry}{domain}; Secure; SameSite=Lax")
}

fn expire_legacy_csrf_cookie(response: &mut Response) {
    response.headers_mut().append(
        SET_COOKIE,
        HeaderValue::from_static("tactica_csrf=; Path=/api/v1; Max-Age=0; Secure; SameSite=Lax"),
    );
}

fn issue_response(
    issued: IssuedSession,
    cookie_mode: bool,
    csrf_cookie_domain: Option<&str>,
) -> Response {
    let body = AuthResponse {
        account_id: issued.current.account_id.to_string(),
        email_verified: issued.current.email_verified,
        session_token: (!cookie_mode).then_some(issued.token.clone()),
    };
    let mut response = Json(body).into_response();
    if cookie_mode {
        response.headers_mut().append(
            SET_COOKIE,
            HeaderValue::from_str(&format!(
                "{SESSION_COOKIE}={}; Path=/api/v1; HttpOnly; Secure; SameSite=Lax",
                issued.token
            ))
            .unwrap(),
        );
        response.headers_mut().append(
            SET_COOKIE,
            HeaderValue::from_str(&csrf_cookie(&issued.csrf_token, csrf_cookie_domain, false))
                .unwrap(),
        );
    } else {
        response.headers_mut().append(
            SET_COOKIE,
            HeaderValue::from_static(
                "tactica_session=; Path=/api/v1; Max-Age=0; HttpOnly; Secure; SameSite=Lax",
            ),
        );
        response.headers_mut().append(
            SET_COOKIE,
            HeaderValue::from_str(&csrf_cookie("", csrf_cookie_domain, true)).unwrap(),
        );
    }
    response
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Response> {
    let current = authenticated(&state, &headers, true).await?;
    state.identity_service.logout(&current).await?;
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().append(
        SET_COOKIE,
        HeaderValue::from_static(
            "tactica_session=; Path=/api/v1; Max-Age=0; HttpOnly; Secure; SameSite=Lax",
        ),
    );
    response.headers_mut().append(
        SET_COOKIE,
        HeaderValue::from_str(&csrf_cookie("", state.csrf_cookie_domain.as_deref(), true)).unwrap(),
    );
    Ok(response)
}

async fn session(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Response> {
    let cookie_auth = matches!(credentials(&headers)?, Some(Transport::Cookie(_)));
    let csrf_token = cookie(&headers, CSRF_COOKIE).map(str::to_owned);
    let current = authenticated(&state, &headers, false).await?;
    let mut response = Json(SessionResponse {
        account_id: current.account_id.to_string(),
        email_verified: current.email_verified,
        session: SessionTimes {
            created_at: current.created_at,
            idle_expires_at: current.idle_expires_at,
            absolute_expires_at: current.absolute_expires_at,
        },
    })
    .into_response();
    if cookie_auth {
        if let Some(csrf_token) = csrf_token {
            response.headers_mut().append(
                SET_COOKIE,
                HeaderValue::from_str(&csrf_cookie(
                    &csrf_token,
                    state.csrf_cookie_domain.as_deref(),
                    false,
                ))
                .expect("validated CSRF cookie attributes"),
            );
        }
        expire_legacy_csrf_cookie(&mut response);
    }
    Ok(response)
}

async fn verify_email(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<VerificationBody>,
) -> ApiResult<StatusCode> {
    let current = authenticated(&state, &headers, true).await?;
    state
        .identity_service
        .verify_email(&current, &body.code)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn resend_verification(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    let current = authenticated(&state, &headers, true).await?;
    state.identity_service.resend_verification(&current).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{
        body::Body,
        http::{
            HeaderValue, Request, StatusCode,
            header::{AUTHORIZATION, ORIGIN, SET_COOKIE},
        },
    };
    use chrono::{TimeDelta, Utc};
    use newtype_uuid::GenericUuid;
    use tactica_module_identity::{
        models::{
            account::{AccountId, EmailAddress},
            session::{CurrentSession, IssuedSession, SessionId},
        },
        ports::{AuthError, IdentityService},
    };
    use tower::ServiceExt;
    use uuid::Uuid;

    use crate::{AppState, config::ApiConfig};

    struct FakeService;

    fn current() -> CurrentSession {
        let now = Utc::now();
        CurrentSession {
            id: SessionId::from_untyped_uuid(Uuid::now_v7()),
            account_id: AccountId::from_untyped_uuid(Uuid::now_v7()),
            email_verified: false,
            created_at: now,
            idle_expires_at: now + TimeDelta::days(30),
            absolute_expires_at: now + TimeDelta::days(180),
        }
    }

    #[async_trait::async_trait]
    impl IdentityService for FakeService {
        async fn register(&self, _: EmailAddress, _: &str) -> Result<IssuedSession, AuthError> {
            Ok(IssuedSession {
                current: current(),
                token: "session-token".into(),
                csrf_token: "csrf-token".into(),
            })
        }
        async fn login(&self, _: EmailAddress, _: &str) -> Result<IssuedSession, AuthError> {
            unreachable!()
        }
        async fn authenticate(&self, token: &str) -> Result<CurrentSession, AuthError> {
            (token == "valid")
                .then(current)
                .ok_or(AuthError::Unauthorized)
        }
        async fn logout(&self, _: &CurrentSession) -> Result<(), AuthError> {
            Ok(())
        }
        async fn validate_csrf(&self, _: &CurrentSession, token: &str) -> Result<(), AuthError> {
            (token == "csrf-token")
                .then_some(())
                .ok_or(AuthError::Unauthorized)
        }
        async fn verify_email(&self, _: &CurrentSession, _: &str) -> Result<(), AuthError> {
            Ok(())
        }
        async fn resend_verification(&self, _: &CurrentSession) -> Result<(), AuthError> {
            Ok(())
        }
    }

    fn app() -> axum::Router {
        crate::router(ApiConfig::default(), AppState::new(Arc::new(FakeService)))
    }

    fn registration() -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/api/v1/auth/register")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"email":"a@example.test","password":"this is a sufficiently long password"}"#,
            ))
            .unwrap()
    }

    #[tokio::test]
    async fn missing_origin_issues_bearer_json() {
        let response = app().oneshot(registration()).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap()["session_token"],
            "session-token"
        );
    }

    #[tokio::test]
    async fn trusted_origin_issues_secure_cookies_without_bearer() {
        let mut request = registration();
        request
            .headers_mut()
            .insert(ORIGIN, HeaderValue::from_static("http://localhost:5173"));
        let response = app().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let cookies: Vec<_> = response
            .headers()
            .get_all(SET_COOKIE)
            .iter()
            .map(|v| v.to_str().unwrap())
            .collect();
        assert!(
            cookies.iter().any(|v| v.contains("HttpOnly")
                && v.contains("Secure")
                && v.contains("SameSite=Lax"))
        );
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert!(
            serde_json::from_slice::<serde_json::Value>(&body)
                .unwrap()
                .get("session_token")
                .is_none()
        );
    }

    #[tokio::test]
    async fn cors_preflight_allows_json_and_csrf_headers() {
        let request = Request::builder()
            .method("OPTIONS")
            .uri("/api/v1/auth/login")
            .header(ORIGIN, "http://localhost:5173")
            .header("access-control-request-method", "POST")
            .header(
                "access-control-request-headers",
                "content-type,x-csrf-token",
            )
            .body(Body::empty())
            .unwrap();
        let response = app().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get("access-control-allow-headers")
                .unwrap(),
            "content-type,x-csrf-token"
        );
    }

    #[test]
    fn csrf_cookie_is_readable_by_the_web_app_domain() {
        assert_eq!(
            super::csrf_cookie("token", Some(".tactica.systems"), false),
            "tactica_csrf=token; Path=/; Domain=.tactica.systems; Secure; SameSite=Lax"
        );
    }

    #[tokio::test]
    async fn untrusted_origin_is_rejected_without_bearer_fallback() {
        let mut request = registration();
        request
            .headers_mut()
            .insert(ORIGIN, HeaderValue::from_static("https://evil.example"));
        assert_eq!(
            app().oneshot(request).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn cookie_and_bearer_are_mutually_exclusive() {
        let request = Request::builder()
            .uri("/api/v1/auth/session")
            .header("cookie", "tactica_session=valid")
            .header(AUTHORIZATION, "Bearer valid")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app().oneshot(request).await.unwrap().status(),
            StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn cookie_logout_requires_csrf_header_and_cookie() {
        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/auth/logout")
            .header("cookie", "tactica_session=valid")
            .header(ORIGIN, "http://localhost:5173")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app().oneshot(request).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn session_migrates_legacy_csrf_cookie_path() {
        let request = Request::builder()
            .uri("/api/v1/auth/session")
            .header("cookie", "tactica_session=valid; tactica_csrf=csrf-token")
            .body(Body::empty())
            .unwrap();
        let response = app().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let cookies: Vec<_> = response
            .headers()
            .get_all(SET_COOKIE)
            .iter()
            .map(|value| value.to_str().unwrap())
            .collect();
        assert!(
            cookies
                .iter()
                .any(|value| value.contains("Path=/;") && value.contains("csrf-token"))
        );
        assert!(
            cookies
                .iter()
                .any(|value| value.contains("Path=/api/v1") && value.contains("Max-Age=0"))
        );
    }
}
