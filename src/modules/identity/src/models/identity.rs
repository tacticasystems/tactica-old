use derive_more::From;
use newtype_uuid_macros::impl_typed_uuid_kinds;
use serde::{Deserialize, Serialize};
use tactica_kernel::{api_error::ApiError, impl_created_at};
use thiserror::Error;

use crate::models::account::AccountId;

impl_typed_uuid_kinds! {
    kinds = {
        Identity = { alias = IdentityId }
    }
}

/// A user account identity's data field
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum IdentityData {
    Password(String),

    #[serde(rename = "oauth")]
    OAuth {
        provider: String,
        sub: String,
    },
}

/// A user account identity, mapping an account to an identity provider.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Identity {
    id: IdentityId,
    account_id: AccountId,
    data: IdentityData,
}

impl Identity {
    pub fn new(id: IdentityId, account_id: AccountId, data: IdentityData) -> Self {
        Self {
            id,
            account_id,
            data,
        }
    }

    pub fn id(&self) -> &IdentityId {
        &self.id
    }

    pub fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    pub fn data(&self) -> &IdentityData {
        &self.data
    }
}

impl_created_at!(Identity);

/// The fields required by the domain to create an [Identity].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, From)]
pub struct CreateIdentityRequest {
    account_id: AccountId,
    password: String,
}

impl CreateIdentityRequest {
    pub fn new(account_id: AccountId, password: String) -> Self {
        Self {
            account_id,
            password,
        }
    }

    pub fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    pub fn password(&self) -> &String {
        &self.password
    }
}

#[derive(Debug, Error)]
pub enum CreateIdentityError {
    #[error("account with ID {0} does not exist")]
    UnknownAccountId(AccountId),

    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

impl ApiError for CreateIdentityError {
    fn status_code(&self) -> http::StatusCode {
        match self {
            CreateIdentityError::UnknownAccountId { .. } => http::StatusCode::FORBIDDEN,
            _ => http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> String {
        match self {
            CreateIdentityError::UnknownAccountId { .. } => {
                "You do not have permission to alter that user's authentication settings.".into()
            }
            _ => "An unknown error occurred".to_string(),
        }
    }

    fn code(&self) -> &'static str {
        match self {
            CreateIdentityError::UnknownAccountId { .. } => "forbidden",
            _ => "unknown_error",
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_password_serialization() {
        let data = super::IdentityData::Password("password123".to_string());
        let json = serde_json::to_string(&data).expect("Failed to serialize IdentityData");
        assert_eq!(json, r#"{"type":"password","value":"password123"}"#);
    }

    #[test]
    fn test_oauth_serialization() {
        let data = super::IdentityData::OAuth {
            provider: "discord".to_string(),
            sub: "abc123".to_string(),
        };
        let json = serde_json::to_string(&data).expect("Failed to serialize IdentityData");
        assert_eq!(
            json,
            r#"{"type":"oauth","value":{"provider":"discord","sub":"abc123"}}"#
        );
    }
}
