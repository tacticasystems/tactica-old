use crate::models::account::AccountId;

#[allow(missing_docs)]
mod ids {
    use newtype_uuid_macros::impl_typed_uuid_kinds;

    impl_typed_uuid_kinds! {
        kinds = { Identity = { alias = IdentityId } }
    }
}
pub use ids::*;

/// A credential or provider used to authenticate an Account.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthenticationMethod {
    /// A locally verified password hash.
    Password {
        /// Argon2id hash encoded in PHC format.
        password_hash: String,
    },
    /// An identity asserted by an external provider.
    External {
        /// Stable provider name.
        provider: String,
        /// Provider-specific subject identifier.
        subject: String,
    },
}

/// An authentication method owned by an Account.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    /// Identity identifier.
    pub id: IdentityId,
    /// Account that owns the Identity.
    pub account_id: AccountId,
    /// Authentication method represented by the Identity.
    pub method: AuthenticationMethod,
}
