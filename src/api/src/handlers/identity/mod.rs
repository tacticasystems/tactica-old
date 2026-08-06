use std::sync::Arc;

use axum::{Json, Router, debug_handler, extract::State, routing::post};
use serde::Deserialize;
use tactica_kernel::api_error::{ApiError, ApiResult};
use tactica_module_identity::{
    models::{
        account::{Account, AccountId, CreateAccountRequest, EmailAddress, EmailAddressError},
        identity::CreateIdentityRequest,
    },
    ports::IdentityService,
};
use thiserror::Error;

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/auth/register", post(create_account))
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct CreateAccountBody {
    email: String,
    password: String,
}

#[derive(Debug, Clone, Error)]
enum ParseCreateAccountRequestError {
    #[error(transparent)]
    EmailAddress(#[from] EmailAddressError),
}

impl ApiError for ParseCreateAccountRequestError {
    fn status_code(&self) -> axum::http::StatusCode {
        match self {
            ParseCreateAccountRequestError::EmailAddress(_) => axum::http::StatusCode::BAD_REQUEST,
        }
    }

    fn message(&self) -> String {
        match self {
            ParseCreateAccountRequestError::EmailAddress(e) => e.to_string(),
        }
    }

    fn code(&self) -> &'static str {
        match self {
            ParseCreateAccountRequestError::EmailAddress(_) => "invalid_email_address",
        }
    }
}

impl CreateAccountBody {
    pub fn into_account_domain(
        &self,
    ) -> Result<CreateAccountRequest, ParseCreateAccountRequestError> {
        let email = EmailAddress::new(&self.email)?;
        Ok(CreateAccountRequest::new(email))
    }

    pub fn into_identity_domain(
        &self,
        account_id: AccountId,
    ) -> Result<CreateIdentityRequest, ParseCreateAccountRequestError> {
        let password = self.password.clone();
        Ok(CreateIdentityRequest::new(account_id, password))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CreateAccountResponse {
    id: String,
}

impl From<&Account> for CreateAccountResponse {
    fn from(account: &Account) -> Self {
        Self {
            id: account.id().to_string(),
        }
    }
}

#[debug_handler]
async fn create_account(
    State(svc): State<Arc<dyn IdentityService>>,
    Json(body): Json<CreateAccountBody>,
) -> ApiResult<Json<CreateAccountResponse>> {
    let domain_req = body.into_account_domain()?;
    let account = svc.create_account(&domain_req).await?;

    let identity_req = body.into_identity_domain(account.id().clone())?;

    svc.create_identity(&identity_req).await?;

    Ok(Json((&account).into()))
}
