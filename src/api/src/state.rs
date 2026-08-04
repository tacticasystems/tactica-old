use std::sync::Arc;

use axum::extract::FromRef;
use tactica_module_identity::ports::IdentityService;

#[derive(Clone)]
pub struct AppState {
    pub(crate) identity_service: Arc<dyn IdentityService>,
}

impl AppState
{
    pub fn new(identity_service: Arc<dyn IdentityService>) -> Self {
        Self { identity_service }
    }
}

impl FromRef<AppState> for Arc<dyn IdentityService>
{
    fn from_ref(state: &AppState) -> Self {
        state.identity_service.clone()
    }
}
