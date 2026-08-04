use anyhow::Context;
use async_trait::async_trait;
use newtype_uuid::GenericUuid;
use tactica_module_identity::{models::{account::{Account, AccountId, CreateAccountError, CreateAccountRequest}, identity::{CreateIdentityError, CreateIdentityRequest, Identity, IdentityId}}, ports::{AccountRepository, IdentityRepository}};
use uuid::Uuid;
use sqlx::{Executor, error::DatabaseError, postgres::PgDatabaseError};

use crate::Postgres;

#[async_trait]
impl AccountRepository for Postgres {
    async fn create_account(&self, req: &CreateAccountRequest) -> Result<Account, CreateAccountError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start Postgres transaction")?;

        let id = Uuid::now_v7();
        let email = req.email().to_string();

        let query = sqlx::query!(
            "INSERT INTO account (account_id, email) VALUES ($1, $2)",
            id,
            email,
        );

        tx.execute(query)
            .await
            .map_err(|e| {
                if let sqlx::Error::Database(db_err) = &e {
                    if db_err.is_unique_violation() {
                        return CreateAccountError::Duplicate { email: req.email().clone() };
                    }
                }

                CreateAccountError::Unknown(e.into())
            })?;

        tx.commit()
            .await
            .context("failed to commit Postgres transaction")?;

        Ok(Account::new(
            AccountId::from_untyped_uuid(id),
            req.email().clone()
        ))
    }
}

#[async_trait]
impl IdentityRepository for Postgres {
    async fn create_identity(&self, req: &CreateIdentityRequest) -> Result<Identity, CreateIdentityError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start Postgres transaction")?;

        let id = Uuid::now_v7();
        let account_id = req.account_id().into_untyped_uuid();

        let query = sqlx::query!(
            "INSERT INTO identity (identity_id, account_id, data) VALUES ($1, $2, '{}')",
            id,
            account_id,
        );

        tx.execute(query)
            .await
            .map_err(|e| CreateIdentityError::Unknown(e.into()))?;

        tx.commit()
            .await
            .context("failed to commit Postgres transaction")?;

        Ok(Identity::new(
            IdentityId::from_untyped_uuid(id),
            req.account_id().clone()
        ))
    }
}
