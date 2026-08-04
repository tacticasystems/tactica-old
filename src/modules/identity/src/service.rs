use async_trait::async_trait;

use crate::{models::account, ports};

#[derive(Debug, Clone)]
pub struct Service<AR, IR>
where
    AR: ports::AccountRepository,
    IR: ports::IdentityRepository,
{
    account_repository: AR,
    _identity_repository: IR,
}

impl<AR, IR> Service<AR, IR>
where
    AR: ports::AccountRepository,
    IR: ports::IdentityRepository,
{
    pub fn new(account_repository: AR, identity_repository: IR) -> Self {
        Self {
            account_repository,
            _identity_repository: identity_repository,
        }
    }
}

#[async_trait]
impl<AR, IR> ports::IdentityService for Service<AR, IR>
where
    AR: ports::AccountRepository,
    IR: ports::IdentityRepository,
{
    async fn create_account(
        &self,
        req: &account::CreateAccountRequest,
    ) -> Result<account::Account, account::CreateAccountError> {
        self.account_repository.create_account(req).await
    }
}
