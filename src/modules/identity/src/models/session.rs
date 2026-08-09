use crate::models::account::AccountId;
use chrono::{DateTime, Utc};

#[allow(missing_docs)]
mod ids {
    use newtype_uuid_macros::impl_typed_uuid_kinds;

    impl_typed_uuid_kinds! {
        kinds = { Session = { alias = SessionId } }
    }
}
pub use ids::*;

/// Validated authentication state for the current request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurrentSession {
    /// Session identifier.
    pub id: SessionId,
    /// Authenticated Account identifier.
    pub account_id: AccountId,
    /// Whether the Account's email has been verified.
    pub email_verified: bool,
    /// Time at which the Session was created.
    pub created_at: DateTime<Utc>,
    /// Current inactivity deadline.
    pub idle_expires_at: DateTime<Utc>,
    /// Maximum Session lifetime deadline.
    pub absolute_expires_at: DateTime<Utc>,
}

/// A newly issued Session and its client credentials.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IssuedSession {
    /// Validated Session metadata.
    pub current: CurrentSession,
    /// Raw opaque Session token shown only to the client.
    pub token: String,
    /// Raw CSRF token shown only to the client.
    pub csrf_token: String,
}
