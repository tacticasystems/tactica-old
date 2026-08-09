use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tactica_kernel::api_error::ApiError;
use thiserror::Error;

use crate::models::{
    account::{AccountId, EmailAddress},
    session::{CurrentSession, IssuedSession, SessionId},
};

/// Errors produced by authentication operations.
#[derive(Debug, Error)]
pub enum AuthError {
    /// The canonical email already belongs to an Account.
    #[error("an account with that email already exists")]
    DuplicateAccount,
    /// The password violates the configured policy.
    #[error("password must contain 15 to 256 UTF-8 bytes")]
    InvalidPassword,
    /// The supplied login credentials are invalid.
    #[error("invalid email or password")]
    InvalidCredentials,
    /// The request has no valid Session.
    #[error("authentication is required")]
    Unauthorized,
    /// The Account must verify its email before proceeding.
    #[error("email verification is required")]
    EmailVerificationRequired,
    /// The verification code is invalid or expired.
    #[error("the verification code is invalid or expired")]
    InvalidVerificationCode,
    /// The verification challenge has exhausted its attempts.
    #[error("too many verification attempts")]
    TooManyVerificationAttempts,
    /// An unexpected infrastructure error occurred.
    #[error("unknown authentication error")]
    Unknown(#[from] anyhow::Error),
}

impl ApiError for AuthError {
    fn status_code(&self) -> http::StatusCode {
        match self {
            Self::DuplicateAccount => http::StatusCode::CONFLICT,
            Self::InvalidPassword | Self::InvalidVerificationCode => http::StatusCode::BAD_REQUEST,
            Self::InvalidCredentials | Self::Unauthorized => http::StatusCode::UNAUTHORIZED,
            Self::EmailVerificationRequired | Self::TooManyVerificationAttempts => {
                http::StatusCode::FORBIDDEN
            }
            Self::Unknown(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> String {
        self.to_string()
    }
    fn code(&self) -> &'static str {
        match self {
            Self::DuplicateAccount => "duplicate_account",
            Self::InvalidPassword => "invalid_password",
            Self::InvalidCredentials => "invalid_credentials",
            Self::Unauthorized => "unauthorized",
            Self::EmailVerificationRequired => "email_verification_required",
            Self::InvalidVerificationCode => "invalid_verification_code",
            Self::TooManyVerificationAttempts => "too_many_verification_attempts",
            Self::Unknown(_) => "unknown_error",
        }
    }
}

/// Values persisted atomically during password registration.
pub struct RegistrationRecord {
    /// Identifier assigned to the Account.
    pub account_id: AccountId,
    /// Canonical Account email.
    pub email: EmailAddress,
    /// Password hash encoded in PHC format.
    pub password_hash: String,
    /// Initial Session to persist.
    pub session: SessionRecord,
    /// Hash of the initial verification code.
    pub challenge_hash: Vec<u8>,
    /// Expiry of the initial verification challenge.
    pub challenge_expires_at: DateTime<Utc>,
}

/// Password credential values loaded during login.
pub struct LoginRecord {
    /// Account associated with the password Identity.
    pub account_id: AccountId,
    /// Persisted password hash.
    pub password_hash: String,
    /// Whether the Account email is verified.
    pub email_verified: bool,
}

/// Values persisted for a newly issued Session.
pub struct SessionRecord {
    /// Session identifier.
    pub id: SessionId,
    /// Authenticated Account identifier.
    pub account_id: AccountId,
    /// Hash of the opaque Session token.
    pub token_hash: Vec<u8>,
    /// Hash of the CSRF token.
    pub csrf_hash: Vec<u8>,
    /// Time at which the Session was created.
    pub created_at: DateTime<Utc>,
    /// Current inactivity deadline.
    pub idle_expires_at: DateTime<Utc>,
    /// Maximum Session lifetime deadline.
    pub absolute_expires_at: DateTime<Utc>,
}

/// Session state loaded from persistent storage.
pub struct StoredSession {
    /// Request-facing Session metadata.
    pub current: CurrentSession,
    /// Time at which the Session was revoked, if any.
    pub revoked_at: Option<DateTime<Utc>>,
}

#[async_trait]
/// Persistence operations required by the authentication service.
pub trait AuthRepository: Send + Sync + 'static {
    /// Atomically persists an Account, password Identity, Session, and challenge.
    async fn register(&self, record: RegistrationRecord) -> Result<(), AuthError>;
    /// Loads password login data by canonical email.
    async fn password_login(&self, email: &EmailAddress) -> Result<Option<LoginRecord>, AuthError>;
    /// Persists a newly issued Session.
    async fn create_session(&self, record: SessionRecord) -> Result<(), AuthError>;
    /// Loads a Session by its opaque-token hash.
    async fn session_by_token_hash(
        &self,
        token_hash: &[u8],
    ) -> Result<Option<StoredSession>, AuthError>;
    /// Extends a Session's inactivity deadline.
    async fn touch_session(
        &self,
        id: &SessionId,
        idle_expires_at: DateTime<Utc>,
    ) -> Result<(), AuthError>;
    /// Revokes a Session at the supplied time.
    async fn revoke_session(&self, id: &SessionId, now: DateTime<Utc>) -> Result<(), AuthError>;
    /// Loads the CSRF-token hash for an active Session.
    async fn csrf_hash(&self, id: &SessionId) -> Result<Option<Vec<u8>>, AuthError>;
    /// Consumes a matching verification challenge and verifies its Account.
    async fn verify_email(
        &self,
        id: &SessionId,
        challenge_hash: &[u8],
        now: DateTime<Utc>,
    ) -> Result<bool, AuthError>;
    /// Replaces the active verification challenge and returns its destination email.
    async fn replace_verification_challenge(
        &self,
        account_id: &AccountId,
        hash: &[u8],
        expires_at: DateTime<Utc>,
    ) -> Result<EmailAddress, AuthError>;
}

#[async_trait]
/// Sends email-verification codes without coupling the domain to a provider.
pub trait VerificationNotifier: Send + Sync + 'static {
    /// Sends a verification code to a canonical email address.
    async fn send_verification_code(
        &self,
        email: &EmailAddress,
        code: &str,
    ) -> Result<(), anyhow::Error>;
}

#[async_trait]
/// Application operations for Account authentication and Sessions.
pub trait IdentityService: Send + Sync + 'static {
    /// Registers an Account with a password and issues its initial Session.
    async fn register(
        &self,
        email: EmailAddress,
        password: &str,
    ) -> Result<IssuedSession, AuthError>;
    /// Verifies password credentials and issues a Session.
    async fn login(&self, email: EmailAddress, password: &str) -> Result<IssuedSession, AuthError>;
    /// Validates an opaque token and returns its current Session.
    async fn authenticate(&self, token: &str) -> Result<CurrentSession, AuthError>;
    /// Revokes the current Session.
    async fn logout(&self, session: &CurrentSession) -> Result<(), AuthError>;
    /// Validates a CSRF token against the current Session.
    async fn validate_csrf(&self, session: &CurrentSession, token: &str) -> Result<(), AuthError>;
    /// Verifies the Account email with a single-use code.
    async fn verify_email(&self, session: &CurrentSession, code: &str) -> Result<(), AuthError>;
    /// Replaces and sends the Account's verification challenge.
    async fn resend_verification(&self, session: &CurrentSession) -> Result<(), AuthError>;
}
