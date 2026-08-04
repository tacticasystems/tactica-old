use derive_more::From;
use std::fmt::{Display, Formatter};

use newtype_uuid_macros::impl_typed_uuid_kinds;
use tactica_kernel::{api_error::ApiError, impl_created_at};
use thiserror::Error;

impl_typed_uuid_kinds! {
    kinds = {
        Account = { alias = AccountId }
    }
}

/// A unique user account within Tactica.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Account {
    id: AccountId,
    email: EmailAddress,
}

impl Account {
    pub fn new(id: AccountId, email: EmailAddress) -> Self {
        Self { id, email }
    }

    pub fn id(&self) -> &AccountId {
        &self.id
    }

    pub fn email(&self) -> &EmailAddress {
        &self.email
    }
}

impl_created_at!(Account);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A valid email address.
pub struct EmailAddress(String);

#[derive(Clone, Debug, Error)]
#[error("{invalid_email} is not a valid email address")]
pub struct EmailAddressError {
    pub invalid_email: String,
}

impl ApiError for EmailAddressError {
    fn status_code(&self) -> http::StatusCode {
        http::StatusCode::BAD_REQUEST
    }

    fn message(&self) -> String {
        format!("{} is not a valid email address", self.invalid_email)
    }

    fn code(&self) -> &'static str {
        "invalid_email_address"
    }
}

impl EmailAddress {
    pub fn new(raw: &str) -> Result<Self, EmailAddressError> {
        let trimmed = raw.trim();
        Self::validate_email_address(trimmed)?;
        Ok(Self(trimmed.to_string()))
    }

    fn validate_email_address(_: &str) -> Result<(), EmailAddressError> {
        // Unimplemented example.
        Ok(())
    }
}

impl Display for EmailAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The fields required by the domain to create an [Account].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, From)]
pub struct CreateAccountRequest {
    email: EmailAddress,
}

impl CreateAccountRequest {
    pub fn new(email: EmailAddress) -> Self {
        Self { email }
    }

    pub fn email(&self) -> &EmailAddress {
        &self.email
    }
}

#[derive(Debug, Error)]
pub enum CreateAccountError {
    #[error("account with email {email} already exists")]
    Duplicate { email: EmailAddress },

    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

impl ApiError for CreateAccountError {
    fn status_code(&self) -> http::StatusCode {
        match self {
            CreateAccountError::Duplicate { .. } => http::StatusCode::CONFLICT,
            _ => http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> String {
        match self {
            CreateAccountError::Duplicate { .. } => {
                "An account with that email already exists".into()
            }
            _ => "An unknown error occurred".to_string(),
        }
    }

    fn code(&self) -> &'static str {
        match self {
            CreateAccountError::Duplicate { .. } => "duplicate_account",
            _ => "unknown_error",
        }
    }
}
