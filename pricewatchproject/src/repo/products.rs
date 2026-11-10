use std::time::Duration;

use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::product::{NewProduct, Product, ProductPatch};

const COLUMNS: &str = "id, user_id, url, name, css_selector, target_price, currency, \
     last_price, last_checked_at, last_error, notified_at, active, created_at";

pub async fn create(db: &PgPool, user_id: Uuid, new: &NewProduct) -> sqlx::Result<Product> {
    let sql = format!(
        "INSERT INTO products (id, user_id, url, name, css_selector, target_price)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING {COLUMNS}"
    );
    sqlx::query_as::<_, Product>(sqlx::AssertSqlSafe(sql))
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(&new.url)
        .bind(&new.name)
        .bind(&new.css_selector)
        .bind(new.target_price)
        .fetch_one(db)
        .await
}

pub async fn list_for_user(db: &PgPool, user_id: Uuid) -> sqlx::Result<Vec<Product>> {
    let sql = format!("SELECT {COLUMNS} FROM products WHERE user_id = $1 ORDER BY created_at DESC");
    sqlx::query_as::<_, Product>(sqlx::AssertSqlSafe(sql))
        .bind(user_id)
        .fetch_all(db)
        .await
}

/// Fetch a product only if it belongs to `user_id`.
pub async fn find_for_user(db: &PgPool, user_id: Uuid, id: Uuid) -> sqlx::Result<Option<Product>> {
    let sql = format!("SELECT {COLUMNS} FROM products WHERE id = $1 AND user_id = $2");
    sqlx::query_as::<_, Product>(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .bind(user_id)
        .fetch_optional(db)
        .await
}

pub async fn update(
    db: &PgPool,
    user_id: Uuid,
    id: Uuid,
    patch: &ProductPatch,
) -> sqlx::Result<Option<Product>> {
    // COALESCE keeps the old value when a field is not provided.
    // Changing the target clears `notified_at` so the new target can alert.
    let sql = format!(
        "UPDATE products SET
            target_price = COALESCE($3, target_price),
            name         = COALESCE($4, name),
            css_selector = COALESCE($5, css_selector),
            active       = COALESCE($6, active),
            notified_at  = CASE WHEN $3 IS NULL THEN notified_at ELSE NULL END
         WHERE id = $1 AND user_id = $2
         RETURNING {COLUMNS}"
    );
    sqlx::query_as::<_, Product>(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .bind(user_id)
        .bind(patch.target_price)
        .bind(&patch.name)
        .bind(&patch.css_selector)
        .bind(patch.active)
        .fetch_optional(db)
        .await
}

pub async fn delete(db: &PgPool, user_id: Uuid, id: Uuid) -> sqlx::Result<bool> {
    let res = sqlx::query("DELETE FROM products WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(db)
        .await?;
    Ok(res.rows_affected() > 0)
}

/// Active products that were never checked, or not checked within `interval`.
/// Oldest first, so a backlog is worked through fairly.
pub async fn due_for_check(
    db: &PgPool,
    interval: Duration,
    limit: i64,
) -> sqlx::Result<Vec<Product>> {
    let sql = format!(
        "SELECT {COLUMNS} FROM products
         WHERE active
           AND (last_checked_at IS NULL
                OR last_checked_at < now() - make_interval(secs => $1))
         ORDER BY last_checked_at NULLS FIRST
         LIMIT $2"
    );
    sqlx::query_as::<_, Product>(sqlx::AssertSqlSafe(sql))
        .bind(interval.as_secs_f64())
        .bind(limit)
        .fetch_all(db)
        .await
}

pub async fn record_success(
    db: &PgPool,
    id: Uuid,
    price: Decimal,
    currency: Option<&str>,
    name: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE products SET
            last_price = $2,
            currency = COALESCE($3, currency),
            name = COALESCE(name, $4),
            last_checked_at = now(),
            last_error = NULL
         WHERE id = $1",
    )
    .bind(id)
    .bind(price)
    .bind(currency)
    .bind(name)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn record_failure(db: &PgPool, id: Uuid, error: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE products SET last_checked_at = now(), last_error = $2 WHERE id = $1")
        .bind(id)
        .bind(error)
        .execute(db)
        .await?;
    Ok(())
}

/// Atomically mark the product as notified. Returns false if another worker
/// already did it, so each drop produces exactly one alert.
pub async fn mark_notified(db: &PgPool, id: Uuid) -> sqlx::Result<bool> {
    let res = sqlx::query(
        "UPDATE products SET notified_at = now() WHERE id = $1 AND notified_at IS NULL",
    )
    .bind(id)
    .execute(db)
    .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn clear_notified(db: &PgPool, id: Uuid) -> sqlx::Result<()> {
    sqlx::query("UPDATE products SET notified_at = NULL WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}
