use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::users::User;

pub async fn create(db: &PgPool, email: &str, password_hash: &str) -> sqlx::Result<User> {
    sqlx::query_as::<_, User>(
        "INSERT INTO users (id, email, password_hash)
         VALUES ($1, $2, $3)
         RETURNING id, email, password_hash, created_at",
    )
    .bind(Uuid::new_v4())
    .bind(email)
    .bind(password_hash)
    .fetch_one(db)
    .await
}

pub async fn find_by_email(db: &PgPool, email: &str) -> sqlx::Result<Option<User>> {
    sqlx::query_as::<_, User>(
        "SELECT id, email, password_hash, created_at FROM users WHERE email = $1",
    )
    .bind(email)
    .fetch_optional(db)
    .await
}

pub async fn find_by_id(db: &PgPool, id: Uuid) -> sqlx::Result<Option<User>> {
    sqlx::query_as::<_, User>(
        "SELECT id, email, password_hash, created_at FROM users WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(db)
    .await
}
