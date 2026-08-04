use async_trait::async_trait;

use crate::models::account::{Account, CreateAccountRequest, CreateAccountError};
use crate::models::identity::{CreateIdentityError, CreateIdentityRequest, Identity};

#[async_trait]
pub trait IdentityService: Send + Sync + 'static {
    /// Create a new [Account]
    ///
    /// # Errors
    ///
    /// - [CreateAccountError::Duplicate] if an [Account] with the same
    ///   [EmailAddress] already exists.
    async fn create_account(&self, req: &CreateAccountRequest) -> Result<Account, CreateAccountError>;
}

/// `AccountRepository` represents a store of Account data.
#[async_trait]
pub trait AccountRepository: Send + Sync + 'static {
    /// Persist a new [Account] to the store.
    ///
    /// # Errors
    ///
    /// - MUST return [CreateAccountError::Duplicate] if an [Account] with the
    ///   same [EmailAddress] already exists.
    async fn create_account(&self, req: &CreateAccountRequest) -> Result<Account, CreateAccountError>;
}

/// `IdentityRepository` represents a store of account identities.
#[async_trait]
pub trait IdentityRepository: Send + Sync + 'static {
    async fn create_identity(&self, req: &CreateIdentityRequest) -> Result<Identity, CreateIdentityError>;
}
