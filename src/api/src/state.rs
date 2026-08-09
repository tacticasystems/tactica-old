use std::sync::Arc;

use axum::extract::FromRef;
use tactica_module_identity::ports::IdentityService;

/// Dependencies shared by HTTP request handlers.
#[derive(Clone)]
pub struct AppState {
    pub(crate) identity_service: Arc<dyn IdentityService>,
    pub(crate) trusted_origins: Arc<Vec<String>>,
}

impl AppState {
    /// Creates state with an authentication service.
    pub fn new(identity_service: Arc<dyn IdentityService>) -> Self {
        Self {
            identity_service,
            trusted_origins: Arc::new(Vec::new()),
        }
    }

    pub(crate) fn with_trusted_origins(mut self, origins: Vec<String>) -> Self {
        self.trusted_origins = Arc::new(origins);
        self
    }
}

impl FromRef<AppState> for Arc<dyn IdentityService> {
    fn from_ref(state: &AppState) -> Self {
        state.identity_service.clone()
    }
}
