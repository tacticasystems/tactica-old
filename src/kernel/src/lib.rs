//! Shared cross-cutting types used by Tactica components.
#![deny(missing_docs)]

/// Standard API error contracts.
pub mod api_error;

/// Implements a UUIDv7-derived `created_at` accessor for a model.
#[macro_export]
macro_rules! impl_created_at {
    ($model:ident) => {
        impl $model {
            pub fn created_at(&self) -> chrono::DateTime<chrono::Utc> {
                use newtype_uuid::GenericUuid;
                self.id
                    .as_untyped_uuid()
                    .get_timestamp()
                    .map(|ts| {
                        let (secs, nanos) = ts.to_unix();
                        chrono::DateTime::from_timestamp(secs as i64, nanos).unwrap()
                    })
                    .unwrap()
            }
        }
    };
}
