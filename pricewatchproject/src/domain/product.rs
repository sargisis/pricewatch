use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Product {
    pub id: Uuid,
    pub user_id: Uuid,
    pub url: String,
    pub name: Option<String>,
    pub css_selector: Option<String>,
    pub target_price: Decimal,
    pub currency: Option<String>,
    pub last_price: Option<Decimal>,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub notified_at: Option<DateTime<Utc>>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct NewProduct {
    pub url: String,
    pub target_price: Decimal,
    pub name: Option<String>,
    pub css_selector: Option<String>,
}

/// Partial update. A field left out of the JSON is not changed.
#[derive(Debug, Default, Deserialize)]
pub struct ProductPatch {
    pub target_price: Option<Decimal>,
    pub name: Option<String>,
    pub css_selector: Option<String>,
    pub active: Option<bool>,
}
