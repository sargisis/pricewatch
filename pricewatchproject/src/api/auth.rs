use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier},
};
use axum::{Json, extract::State, http::StatusCode};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::middleware::AuthUser,
    domain::users::PublicUser,
    error::{AppError, AppResult},
    repo,
    state::AppState,
};

const TOKEN_TTL_DAYS: i64 = 7;
const MIN_PASSWORD_LEN: usize = 8;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: Uuid,
    iat: i64,
    exp: i64,
}

#[derive(Debug, Deserialize)]
pub struct Credentials {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: PublicUser,
}

pub fn issue_token(user_id: Uuid, secret: &str) -> AppResult<String> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id,
        iat: now.timestamp(),
        exp: (now + Duration::days(TOKEN_TTL_DAYS)).timestamp(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("jwt encode: {e}")))
}

pub fn verify_token(token: &str, secret: &str) -> AppResult<Uuid> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims.sub)
    .map_err(|_| AppError::Unauthorized)
}

async fn hash_password(password: String) -> AppResult<String> {
    // Argon2 is deliberately slow; keep it off the async worker threads.
    tokio::task::spawn_blocking(move || {
        Argon2::default()
            .hash_password(password.as_bytes())
            .map(|h| h.to_string())
            .map_err(|e| AppError::Internal(format!("hash: {e}")))
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
}

async fn verify_password(password: String, hash: String) -> AppResult<bool> {
    tokio::task::spawn_blocking(move || {
        Argon2::default()
            .verify_password(password.as_bytes(), hash.as_str())
            .is_ok()
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))
}

fn normalize_email(email: &str) -> AppResult<String> {
    let email = email.trim().to_lowercase();
    let valid = email.len() <= 254
        && email
            .split_once('@')
            .is_some_and(|(user, domain)| !user.is_empty() && domain.contains('.'));
    if !valid {
        return Err(AppError::BadRequest("invalid email".into()));
    }
    Ok(email)
}

pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<Credentials>,
) -> AppResult<(StatusCode, Json<AuthResponse>)> {
    let email = normalize_email(&body.email)?;
    if body.password.chars().count() < MIN_PASSWORD_LEN {
        return Err(AppError::BadRequest(format!(
            "password must be at least {MIN_PASSWORD_LEN} characters"
        )));
    }

    let hash = hash_password(body.password).await?;
    let user = repo::users::create(&state.db, &email, &hash)
        .await
        .map_err(|e| match e.as_database_error() {
            Some(db) if db.is_unique_violation() => {
                AppError::Conflict("email already registered".into())
            }
            _ => AppError::Db(e),
        })?;

    let token = issue_token(user.id, &state.config.jwt_secret)?;
    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            token,
            user: user.into(),
        }),
    ))
}

pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<Credentials>,
) -> AppResult<Json<AuthResponse>> {
    let email = normalize_email(&body.email).map_err(|_| AppError::Unauthorized)?;
    let user = repo::users::find_by_email(&state.db, &email)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if !verify_password(body.password, user.password_hash.clone()).await? {
        return Err(AppError::Unauthorized);
    }

    let token = issue_token(user.id, &state.config.jwt_secret)?;
    Ok(Json(AuthResponse {
        token,
        user: user.into(),
    }))
}

pub async fn me(State(state): State<AppState>, auth: AuthUser) -> AppResult<Json<PublicUser>> {
    let user = repo::users::find_by_id(&state.db, auth.id)
        .await?
        .ok_or(AppError::Unauthorized)?;
    Ok(Json(user.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "test-secret-at-least-16";

    #[test]
    fn token_roundtrip() {
        let id = Uuid::new_v4();
        let token = issue_token(id, SECRET).unwrap();
        assert_eq!(verify_token(&token, SECRET).unwrap(), id);
    }

    #[test]
    fn token_with_wrong_secret_is_rejected() {
        let token = issue_token(Uuid::new_v4(), SECRET).unwrap();
        assert!(verify_token(&token, "another-secret-value").is_err());
    }

    #[test]
    fn email_validation() {
        assert_eq!(normalize_email("  Bob@Mail.com ").unwrap(), "bob@mail.com");
        assert!(normalize_email("nope").is_err());
        assert!(normalize_email("@x.com").is_err());
    }

    #[tokio::test]
    async fn password_hash_roundtrip() {
        let hash = hash_password("hunter4242".into()).await.unwrap();
        assert!(
            verify_password("hunter4242".into(), hash.clone())
                .await
                .unwrap()
        );
        assert!(
            !verify_password("wrong-password".into(), hash)
                .await
                .unwrap()
        );
    }
}
