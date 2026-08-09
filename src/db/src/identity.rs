use anyhow::Context;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use newtype_uuid::GenericUuid;
use sqlx::Row;
use tactica_module_identity::{
    models::{
        account::{AccountId, EmailAddress},
        session::{CurrentSession, SessionId},
    },
    ports::{
        AuthError, AuthRepository, LoginRecord, RegistrationRecord, SessionRecord, StoredSession,
    },
};
use uuid::Uuid;

use crate::Postgres;

fn unknown(error: sqlx::Error) -> AuthError {
    AuthError::Unknown(error.into())
}

#[async_trait]
impl AuthRepository for Postgres {
    async fn register(&self, record: RegistrationRecord) -> Result<(), AuthError> {
        let mut tx = self.pool.begin().await.map_err(unknown)?;
        let account_id = record.account_id.into_untyped_uuid();
        let result = sqlx::query("insert into account (account_id, email) values ($1, $2)")
            .bind(account_id)
            .bind(record.email.to_string())
            .execute(&mut *tx)
            .await;
        if let Err(error) = result {
            if error
                .as_database_error()
                .is_some_and(|e| e.is_unique_violation())
            {
                return Err(AuthError::DuplicateAccount);
            }
            return Err(unknown(error));
        }
        sqlx::query("insert into identity (identity_id, account_id, kind, password_hash) values ($1, $2, 'password', $3)")
            .bind(Uuid::now_v7()).bind(account_id).bind(record.password_hash).execute(&mut *tx).await.map_err(unknown)?;
        insert_session(&mut tx, record.session).await?;
        sqlx::query("insert into email_verification_challenge (account_id, code_hash, expires_at) values ($1, $2, $3)")
            .bind(account_id).bind(record.challenge_hash).bind(record.challenge_expires_at).execute(&mut *tx).await.map_err(unknown)?;
        tx.commit()
            .await
            .context("failed to commit registration transaction")
            .map_err(AuthError::Unknown)
    }

    async fn password_login(&self, email: &EmailAddress) -> Result<Option<LoginRecord>, AuthError> {
        let row = sqlx::query("select a.account_id, i.password_hash, a.email_verified_at is not null as email_verified from account a join identity i on i.account_id = a.account_id and i.kind = 'password' where a.email = $1")
            .bind(email.to_string()).fetch_optional(&self.pool).await.map_err(unknown)?;
        Ok(row.map(|row| LoginRecord {
            account_id: AccountId::from_untyped_uuid(row.get("account_id")),
            password_hash: row.get("password_hash"),
            email_verified: row.get("email_verified"),
        }))
    }

    async fn create_session(&self, record: SessionRecord) -> Result<(), AuthError> {
        let mut tx = self.pool.begin().await.map_err(unknown)?;
        insert_session(&mut tx, record).await?;
        tx.commit().await.map_err(unknown)?;
        Ok(())
    }

    async fn session_by_token_hash(
        &self,
        token_hash: &[u8],
    ) -> Result<Option<StoredSession>, AuthError> {
        let row = sqlx::query("select s.session_id, s.account_id, s.created_at, s.idle_expires_at, s.absolute_expires_at, s.revoked_at, a.email_verified_at is not null as email_verified from session s join account a on a.account_id = s.account_id where s.token_hash = $1")
            .bind(token_hash).fetch_optional(&self.pool).await.map_err(unknown)?;
        Ok(row.map(|row| StoredSession {
            current: CurrentSession {
                id: SessionId::from_untyped_uuid(row.get("session_id")),
                account_id: AccountId::from_untyped_uuid(row.get("account_id")),
                email_verified: row.get("email_verified"),
                created_at: row.get("created_at"),
                idle_expires_at: row.get("idle_expires_at"),
                absolute_expires_at: row.get("absolute_expires_at"),
            },
            revoked_at: row.get("revoked_at"),
        }))
    }

    async fn touch_session(
        &self,
        id: &SessionId,
        idle_expires_at: DateTime<Utc>,
    ) -> Result<(), AuthError> {
        sqlx::query("update session set idle_expires_at = least($2, absolute_expires_at) where session_id = $1 and revoked_at is null")
            .bind(id.into_untyped_uuid()).bind(idle_expires_at).execute(&self.pool).await.map_err(unknown)?;
        Ok(())
    }

    async fn revoke_session(&self, id: &SessionId, now: DateTime<Utc>) -> Result<(), AuthError> {
        sqlx::query(
            "update session set revoked_at = coalesce(revoked_at, $2) where session_id = $1",
        )
        .bind(id.into_untyped_uuid())
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(unknown)?;
        Ok(())
    }

    async fn csrf_hash(&self, id: &SessionId) -> Result<Option<Vec<u8>>, AuthError> {
        sqlx::query_scalar(
            "select csrf_token_hash from session where session_id = $1 and revoked_at is null",
        )
        .bind(id.into_untyped_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(unknown)
    }

    async fn verify_email(
        &self,
        id: &SessionId,
        challenge_hash: &[u8],
        now: DateTime<Utc>,
    ) -> Result<bool, AuthError> {
        let mut tx = self.pool.begin().await.map_err(unknown)?;
        let row = sqlx::query("select c.challenge_id, c.account_id, c.code_hash = $2 as matches, c.failed_attempts, c.expires_at from email_verification_challenge c join session s on s.account_id = c.account_id where s.session_id = $1 and c.consumed_at is null for update of c")
            .bind(id.into_untyped_uuid()).bind(challenge_hash).fetch_optional(&mut *tx).await.map_err(unknown)?;
        let Some(row) = row else {
            return Ok(false);
        };
        let challenge_id: Uuid = row.get("challenge_id");
        let matches: bool = row.get("matches");
        let attempts: i16 = row.get("failed_attempts");
        let expires_at: DateTime<Utc> = row.get("expires_at");
        if !matches || attempts >= 5 || now >= expires_at {
            sqlx::query("update email_verification_challenge set failed_attempts = least(failed_attempts + 1, 5), consumed_at = case when failed_attempts + 1 >= 5 then $2 else consumed_at end where challenge_id = $1")
                .bind(challenge_id).bind(now).execute(&mut *tx).await.map_err(unknown)?;
            tx.commit().await.map_err(unknown)?;
            return Ok(false);
        }
        let account_id: Uuid = row.get("account_id");
        sqlx::query("update account set email_verified_at = coalesce(email_verified_at, $2) where account_id = $1")
            .bind(account_id).bind(now).execute(&mut *tx).await.map_err(unknown)?;
        sqlx::query(
            "update email_verification_challenge set consumed_at = $2 where challenge_id = $1",
        )
        .bind(challenge_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(unknown)?;
        tx.commit().await.map_err(unknown)?;
        Ok(true)
    }

    async fn replace_verification_challenge(
        &self,
        account_id: &AccountId,
        hash: &[u8],
        expires_at: DateTime<Utc>,
    ) -> Result<EmailAddress, AuthError> {
        let mut tx = self.pool.begin().await.map_err(unknown)?;
        let raw_id = account_id.into_untyped_uuid();
        sqlx::query("update email_verification_challenge set consumed_at = now() where account_id = $1 and consumed_at is null")
            .bind(raw_id).execute(&mut *tx).await.map_err(unknown)?;
        sqlx::query("insert into email_verification_challenge (account_id, code_hash, expires_at) values ($1, $2, $3)")
            .bind(raw_id).bind(hash).bind(expires_at).execute(&mut *tx).await.map_err(unknown)?;
        let email: String = sqlx::query_scalar("select email from account where account_id = $1")
            .bind(raw_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(unknown)?;
        tx.commit().await.map_err(unknown)?;
        EmailAddress::new(&email).map_err(|e| AuthError::Unknown(e.into()))
    }
}

async fn insert_session(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    record: SessionRecord,
) -> Result<(), AuthError> {
    sqlx::query("insert into session (session_id, account_id, token_hash, csrf_token_hash, created_at, idle_expires_at, absolute_expires_at) values ($1, $2, $3, $4, $5, $6, $7)")
        .bind(record.id.into_untyped_uuid()).bind(record.account_id.into_untyped_uuid())
        .bind(record.token_hash).bind(record.csrf_hash).bind(record.created_at)
        .bind(record.idle_expires_at).bind(record.absolute_expires_at)
        .execute(&mut **tx).await.map_err(unknown)?;
    Ok(())
}
