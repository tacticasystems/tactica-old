use std::time::Duration;

use argon2::{
    Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version,
    password_hash::SaltString,
};
use chrono::{DateTime, TimeDelta, Utc};
use newtype_uuid::GenericUuid;
use rand::Rng;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    models::{
        account::{AccountId, EmailAddress},
        session::{CurrentSession, IssuedSession, SessionId},
    },
    ports::{
        AuthError, AuthRepository, IdentityService, LoginRecord, RegistrationRecord, SessionRecord,
        VerificationNotifier,
    },
};

/// Configures Session and verification lifetimes.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    /// Inactivity duration after which a Session expires.
    pub idle_timeout: Duration,
    /// Maximum lifetime of a Session.
    pub absolute_timeout: Duration,
    /// Minimum interval between sliding-expiry writes.
    pub refresh_interval: Duration,
    /// Lifetime of an email-verification challenge.
    pub verification_timeout: Duration,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            idle_timeout: Duration::from_secs(30 * 24 * 60 * 60),
            absolute_timeout: Duration::from_secs(180 * 24 * 60 * 60),
            refresh_interval: Duration::from_secs(60 * 60),
            verification_timeout: Duration::from_secs(24 * 60 * 60),
        }
    }
}

/// Supplies the current time to authentication operations.
pub trait Clock: Send + Sync + 'static {
    /// Returns the current UTC time.
    fn now(&self) -> DateTime<Utc>;
}
/// Clock backed by the system UTC time.
#[derive(Clone, Copy, Debug)]
pub struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Coordinates authentication policy with persistence and notification ports.
pub struct Service<R, N, C = SystemClock> {
    repository: R,
    notifier: N,
    clock: C,
    config: AuthConfig,
}

impl<R, N> Service<R, N, SystemClock> {
    /// Creates a service backed by the system clock.
    pub fn new(repository: R, notifier: N, config: AuthConfig) -> Self {
        Self {
            repository,
            notifier,
            clock: SystemClock,
            config,
        }
    }
}

impl<R, N, C> Service<R, N, C> {
    /// Creates a service with an explicit clock for deterministic callers.
    pub fn with_clock(repository: R, notifier: N, clock: C, config: AuthConfig) -> Self {
        Self {
            repository,
            notifier,
            clock,
            config,
        }
    }

    fn hash(value: &str) -> Vec<u8> {
        Sha256::digest(value.as_bytes()).to_vec()
    }
    fn token() -> String {
        hex::encode(rand::random::<[u8; 32]>())
    }
    fn code() -> String {
        const ALPHABET: &[u8] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";
        let mut rng = rand::rng();
        (0..6)
            .map(|_| ALPHABET[rng.random_range(0..ALPHABET.len())] as char)
            .collect()
    }
    fn add(now: DateTime<Utc>, duration: Duration) -> DateTime<Utc> {
        now + TimeDelta::from_std(duration).expect("authentication duration must fit chrono")
    }

    fn argon2() -> Result<Argon2<'static>, AuthError> {
        let params = Params::new(64 * 1024, 3, 4, None)
            .map_err(|e| AuthError::Unknown(anyhow::anyhow!(e)))?;
        Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
    }

    fn new_session(
        &self,
        account_id: AccountId,
        now: DateTime<Utc>,
    ) -> (IssuedSession, SessionRecord) {
        let token = Self::token();
        let csrf_token = Self::token();
        let current = CurrentSession {
            id: SessionId::from_untyped_uuid(Uuid::now_v7()),
            account_id,
            email_verified: false,
            created_at: now,
            idle_expires_at: Self::add(now, self.config.idle_timeout),
            absolute_expires_at: Self::add(now, self.config.absolute_timeout),
        };
        let record = SessionRecord {
            id: current.id,
            account_id,
            token_hash: Self::hash(&token),
            csrf_hash: Self::hash(&csrf_token),
            created_at: now,
            idle_expires_at: current.idle_expires_at,
            absolute_expires_at: current.absolute_expires_at,
        };
        (
            IssuedSession {
                current,
                token,
                csrf_token,
            },
            record,
        )
    }
}

#[async_trait::async_trait]
impl<R, N, C> IdentityService for Service<R, N, C>
where
    R: AuthRepository,
    N: VerificationNotifier,
    C: Clock,
{
    async fn register(
        &self,
        email: EmailAddress,
        password: &str,
    ) -> Result<IssuedSession, AuthError> {
        if password.chars().count() < 15 || password.len() > 256 {
            return Err(AuthError::InvalidPassword);
        }
        let salt = SaltString::generate(&mut rand_core::OsRng);
        let hash = Self::argon2()?
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AuthError::Unknown(anyhow::anyhow!(e)))?
            .to_string();
        let now = self.clock.now();
        let account_id = AccountId::from_untyped_uuid(Uuid::now_v7());
        let (issued, session) = self.new_session(account_id, now);
        let code = Self::code();
        self.repository
            .register(RegistrationRecord {
                account_id,
                email: email.clone(),
                password_hash: hash,
                session,
                challenge_hash: Self::hash(&code),
                challenge_expires_at: Self::add(now, self.config.verification_timeout),
            })
            .await?;
        self.notifier
            .send_verification_code(&email, &code)
            .await
            .map_err(AuthError::Unknown)?;
        Ok(issued)
    }

    async fn login(&self, email: EmailAddress, password: &str) -> Result<IssuedSession, AuthError> {
        let LoginRecord {
            account_id,
            password_hash,
            email_verified,
        } = self
            .repository
            .password_login(&email)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;
        let parsed = PasswordHash::new(&password_hash)
            .map_err(|e| AuthError::Unknown(anyhow::anyhow!(e)))?;
        if Self::argon2()?
            .verify_password(password.as_bytes(), &parsed)
            .is_err()
        {
            return Err(AuthError::InvalidCredentials);
        }
        let now = self.clock.now();
        let (mut issued, record) = self.new_session(account_id, now);
        issued.current.email_verified = email_verified;
        self.repository.create_session(record).await?;
        Ok(issued)
    }

    async fn authenticate(&self, token: &str) -> Result<CurrentSession, AuthError> {
        let now = self.clock.now();
        let stored = self
            .repository
            .session_by_token_hash(&Self::hash(token))
            .await?
            .ok_or(AuthError::Unauthorized)?;
        if stored.revoked_at.is_some()
            || now >= stored.current.idle_expires_at
            || now >= stored.current.absolute_expires_at
        {
            return Err(AuthError::Unauthorized);
        }
        let mut current = stored.current;
        let refresh_at = current.idle_expires_at
            - TimeDelta::from_std(self.config.idle_timeout - self.config.refresh_interval).unwrap();
        if now >= refresh_at {
            current.idle_expires_at =
                Self::add(now, self.config.idle_timeout).min(current.absolute_expires_at);
            self.repository
                .touch_session(&current.id, current.idle_expires_at)
                .await?;
        }
        Ok(current)
    }

    async fn logout(&self, session: &CurrentSession) -> Result<(), AuthError> {
        self.repository
            .revoke_session(&session.id, self.clock.now())
            .await
    }

    async fn validate_csrf(&self, session: &CurrentSession, token: &str) -> Result<(), AuthError> {
        let stored = self
            .repository
            .csrf_hash(&session.id)
            .await?
            .ok_or(AuthError::Unauthorized)?;
        if stored == Self::hash(token) {
            Ok(())
        } else {
            Err(AuthError::Unauthorized)
        }
    }

    async fn verify_email(&self, session: &CurrentSession, code: &str) -> Result<(), AuthError> {
        let normalized = code.trim().to_uppercase();
        if self
            .repository
            .verify_email(&session.id, &Self::hash(&normalized), self.clock.now())
            .await?
        {
            Ok(())
        } else {
            Err(AuthError::InvalidVerificationCode)
        }
    }

    async fn resend_verification(&self, session: &CurrentSession) -> Result<(), AuthError> {
        if session.email_verified {
            return Ok(());
        }
        let code = Self::code();
        let now = self.clock.now();
        let email = self
            .repository
            .replace_verification_challenge(
                &session.account_id,
                &Self::hash(&code),
                Self::add(now, self.config.verification_timeout),
            )
            .await?;
        self.notifier
            .send_verification_code(&email, &code)
            .await
            .map_err(AuthError::Unknown)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use chrono::{TimeDelta, TimeZone};
    use newtype_uuid::GenericUuid;
    use uuid::Uuid;

    use super::*;
    use crate::ports::{RegistrationRecord, StoredSession};

    #[derive(Clone)]
    struct TestClock(Arc<Mutex<DateTime<Utc>>>);
    impl Clock for TestClock {
        fn now(&self) -> DateTime<Utc> {
            *self.0.lock().unwrap()
        }
    }

    type StoredTestSession = Option<(Vec<u8>, StoredSession)>;

    #[derive(Clone, Default)]
    struct TestRepo(Arc<Mutex<StoredTestSession>>);

    #[async_trait::async_trait]
    impl AuthRepository for TestRepo {
        async fn register(&self, _: RegistrationRecord) -> Result<(), AuthError> {
            unreachable!()
        }
        async fn password_login(&self, _: &EmailAddress) -> Result<Option<LoginRecord>, AuthError> {
            unreachable!()
        }
        async fn create_session(&self, _: SessionRecord) -> Result<(), AuthError> {
            unreachable!()
        }
        async fn session_by_token_hash(
            &self,
            hash: &[u8],
        ) -> Result<Option<StoredSession>, AuthError> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .as_ref()
                .and_then(|(expected, stored)| {
                    (expected == hash).then_some(StoredSession {
                        current: stored.current.clone(),
                        revoked_at: stored.revoked_at,
                    })
                }))
        }
        async fn touch_session(
            &self,
            id: &SessionId,
            expiry: DateTime<Utc>,
        ) -> Result<(), AuthError> {
            if let Some((_, stored)) = self.0.lock().unwrap().as_mut() {
                assert_eq!(&stored.current.id, id);
                stored.current.idle_expires_at = expiry;
            }
            Ok(())
        }
        async fn revoke_session(&self, _: &SessionId, _: DateTime<Utc>) -> Result<(), AuthError> {
            unreachable!()
        }
        async fn csrf_hash(&self, _: &SessionId) -> Result<Option<Vec<u8>>, AuthError> {
            unreachable!()
        }
        async fn verify_email(
            &self,
            _: &SessionId,
            _: &[u8],
            _: DateTime<Utc>,
        ) -> Result<bool, AuthError> {
            unreachable!()
        }
        async fn replace_verification_challenge(
            &self,
            _: &AccountId,
            _: &[u8],
            _: DateTime<Utc>,
        ) -> Result<EmailAddress, AuthError> {
            unreachable!()
        }
    }

    struct NoopNotifier;
    #[async_trait::async_trait]
    impl VerificationNotifier for NoopNotifier {
        async fn send_verification_code(
            &self,
            _: &EmailAddress,
            _: &str,
        ) -> Result<(), anyhow::Error> {
            Ok(())
        }
    }

    fn fixture() -> (
        Service<TestRepo, NoopNotifier, TestClock>,
        TestRepo,
        TestClock,
        String,
    ) {
        let now = Utc.with_ymd_and_hms(2026, 8, 8, 12, 0, 0).unwrap();
        let clock = TestClock(Arc::new(Mutex::new(now)));
        let repo = TestRepo::default();
        let token = "session-token".to_owned();
        let current = CurrentSession {
            id: SessionId::from_untyped_uuid(Uuid::now_v7()),
            account_id: AccountId::from_untyped_uuid(Uuid::now_v7()),
            email_verified: true,
            created_at: now,
            idle_expires_at: now + TimeDelta::days(30),
            absolute_expires_at: now + TimeDelta::days(180),
        };
        *repo.0.lock().unwrap() = Some((
            Service::<TestRepo, NoopNotifier, TestClock>::hash(&token),
            StoredSession {
                current,
                revoked_at: None,
            },
        ));
        let service = Service::with_clock(
            repo.clone(),
            NoopNotifier,
            clock.clone(),
            AuthConfig::default(),
        );
        (service, repo, clock, token)
    }

    #[tokio::test]
    async fn validates_and_slides_an_active_session() {
        let (service, repo, clock, token) = fixture();
        *clock.0.lock().unwrap() += TimeDelta::days(30) - TimeDelta::minutes(30);
        let current = service.authenticate(&token).await.unwrap();
        assert_eq!(current.idle_expires_at, clock.now() + TimeDelta::days(30));
        assert_eq!(
            repo.0
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .1
                .current
                .idle_expires_at,
            current.idle_expires_at
        );
    }

    #[tokio::test]
    async fn rejects_idle_expiry() {
        let (service, _, clock, token) = fixture();
        *clock.0.lock().unwrap() += TimeDelta::days(30);
        assert!(matches!(
            service.authenticate(&token).await,
            Err(AuthError::Unauthorized)
        ));
    }

    #[tokio::test]
    async fn rejects_absolute_expiry_even_when_idle_deadline_is_later() {
        let (service, repo, clock, token) = fixture();
        repo.0
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .1
            .current
            .idle_expires_at += TimeDelta::days(200);
        *clock.0.lock().unwrap() += TimeDelta::days(180);
        assert!(matches!(
            service.authenticate(&token).await,
            Err(AuthError::Unauthorized)
        ));
    }

    #[test]
    fn verification_codes_are_unambiguous() {
        for _ in 0..100 {
            let code = Service::<TestRepo, NoopNotifier, TestClock>::code();
            assert_eq!(code.len(), 6);
            assert!(
                code.bytes()
                    .all(|byte| b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ".contains(&byte))
            );
        }
    }
}
