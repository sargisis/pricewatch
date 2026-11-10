//! Authentication extractor: add `AuthUser` to a handler's arguments and the
//! request must carry a valid `Authorization: Bearer <jwt>` header.

use axum::{extract::FromRequestParts, http::request::Parts};
use uuid::Uuid;

use crate::{api::auth::verify_token, error::AppError, state::AppState};

#[derive(Debug, Clone, Copy)]
pub struct AuthUser {
    pub id: Uuid,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::Unauthorized)?;

        let token = header
            .strip_prefix("Bearer ")
            .or_else(|| header.strip_prefix("bearer "))
            .ok_or(AppError::Unauthorized)?;

        let id = verify_token(token.trim(), &state.config.jwt_secret)?;
        Ok(AuthUser { id })
    }
}
