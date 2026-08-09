use std::fmt::{Display, Formatter};

use newtype_uuid::GenericUuid;
use tactica_kernel::api_error::ApiError;
use thiserror::Error;

#[allow(missing_docs)]
mod ids {
    use newtype_uuid_macros::impl_typed_uuid_kinds;

    impl_typed_uuid_kinds! {
        kinds = { Account = { alias = AccountId } }
    }
}
pub use ids::*;

/// An application-wide authentication principal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Account {
    id: AccountId,
    email: EmailAddress,
    email_verified: bool,
}

impl Account {
    /// Creates an Account from persisted values.
    pub fn new(id: AccountId, email: EmailAddress, email_verified: bool) -> Self {
        Self {
            id,
            email,
            email_verified,
        }
    }

    /// Returns the Account identifier.
    pub fn id(&self) -> &AccountId {
        &self.id
    }
    /// Returns the Account's canonical email address.
    pub fn email(&self) -> &EmailAddress {
        &self.email
    }
    /// Returns whether ownership of the email has been verified.
    pub fn email_verified(&self) -> bool {
        self.email_verified
    }
}

impl Account {
    /// Returns the creation time encoded in the UUIDv7 identifier.
    pub fn created_at(&self) -> chrono::DateTime<chrono::Utc> {
        let (secs, nanos) = self.id.as_untyped_uuid().get_timestamp().unwrap().to_unix();
        chrono::DateTime::from_timestamp(secs as i64, nanos).unwrap()
    }
}

/// A validated, trimmed, lowercase email address.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EmailAddress(String);

/// Error returned when an email address is invalid.
#[derive(Clone, Debug, Error)]
#[error("{invalid_email} is not a valid email address")]
pub struct EmailAddressError {
    /// The rejected raw email address.
    pub invalid_email: String,
}

impl ApiError for EmailAddressError {
    fn status_code(&self) -> http::StatusCode {
        http::StatusCode::BAD_REQUEST
    }
    fn message(&self) -> String {
        self.to_string()
    }
    fn code(&self) -> &'static str {
        "invalid_email_address"
    }
}

impl EmailAddress {
    /// Validates and canonicalises a raw email address.
    pub fn new(raw: &str) -> Result<Self, EmailAddressError> {
        let canonical = raw.trim().to_lowercase();
        let valid = canonical.len() <= 254
            && canonical.split_once('@').is_some_and(|(local, domain)| {
                !local.is_empty() && !domain.is_empty() && domain.contains('.')
            });
        if !valid {
            return Err(EmailAddressError {
                invalid_email: raw.to_owned(),
            });
        }
        Ok(Self(canonical))
    }
}

impl Display for EmailAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::EmailAddress;

    #[test]
    fn canonicalises_email() {
        assert_eq!(
            EmailAddress::new("  Example@TACTICA.test ")
                .unwrap()
                .to_string(),
            "example@tactica.test"
        );
    }
}
