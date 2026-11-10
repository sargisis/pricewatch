use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::price::{PricePoint, PriceStats};

pub async fn insert(
    db: &PgPool,
    product_id: Uuid,
    price: Decimal,
    currency: Option<&str>,
) -> sqlx::Result<PricePoint> {
    sqlx::query_as::<_, PricePoint>(
        "INSERT INTO price_history (product_id, price, currency)
         VALUES ($1, $2, $3)
         RETURNING id, product_id, price, currency, scraped_at",
    )
    .bind(product_id)
    .bind(price)
    .bind(currency)
    .fetch_one(db)
    .await
}

/// Most recent points first.
pub async fn history(db: &PgPool, product_id: Uuid, limit: i64) -> sqlx::Result<Vec<PricePoint>> {
    sqlx::query_as::<_, PricePoint>(
        "SELECT id, product_id, price, currency, scraped_at
         FROM price_history
         WHERE product_id = $1
         ORDER BY scraped_at DESC
         LIMIT $2",
    )
    .bind(product_id)
    .bind(limit)
    .fetch_all(db)
    .await
}

pub async fn stats(db: &PgPool, product_id: Uuid) -> sqlx::Result<PriceStats> {
    sqlx::query_as::<_, PriceStats>(
        "SELECT MIN(price) AS min_price,
                MAX(price) AS max_price,
                ROUND(AVG(price), 2) AS avg_price,
                COUNT(*) AS samples
         FROM price_history
         WHERE product_id = $1",
    )
    .bind(product_id)
    .fetch_one(db)
    .await
}
