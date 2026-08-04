use derive_more::From;
use newtype_uuid_macros::impl_typed_uuid_kinds;
use tactica_kernel::impl_created_at;
use thiserror::Error;

use crate::models::account::AccountId;

impl_typed_uuid_kinds! {
    kinds = {
        Identity = { alias = IdentityId }
    }
}

/// A user account identity, mapping an account to an identity provider.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Identity {
    id: IdentityId,
    account_id: AccountId,
}

impl Identity {
    pub fn new(id: IdentityId, account_id: AccountId) -> Self {
        Self { id, account_id }
    }

    pub fn id(&self) -> &IdentityId {
        &self.id
    }

    pub fn account_id(&self) -> &AccountId {
        &self.account_id
    }
}

impl_created_at!(Identity);

/// The fields required by the domain to create an [Identity].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, From)]
pub struct CreateIdentityRequest {
    account_id: AccountId,
}

impl CreateIdentityRequest {
    pub fn new(account_id: AccountId) -> Self {
        Self { account_id }
    }

    pub fn account_id(&self) -> &AccountId {
        &self.account_id
    }
}

#[derive(Debug, Error)]
pub enum CreateIdentityError {
    #[error("account with ID {0} does not exist")]
    UnknownAccountId(AccountId),

    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}
