use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PricePoint {
    pub id: i64,
    pub product_id: Uuid,
    pub price: Decimal,
    pub currency: Option<String>,
    pub scraped_at: DateTime<Utc>,
}

/// Summary returned alongside history.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PriceStats {
    pub min_price: Option<Decimal>,
    pub max_price: Option<Decimal>,
    pub avg_price: Option<Decimal>,
    pub samples: i64,
}
