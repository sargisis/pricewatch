use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::middleware::AuthUser,
    domain::price::{PricePoint, PriceStats},
    error::{AppError, AppResult},
    repo,
    state::AppState,
};

const DEFAULT_LIMIT: i64 = 100;
const MAX_LIMIT: i64 = 1000;

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct HistoryResponse {
    pub product_id: Uuid,
    pub stats: PriceStats,
    /// Newest first.
    pub points: Vec<PricePoint>,
}

pub async fn history(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Query(q): Query<HistoryQuery>,
) -> AppResult<Json<HistoryResponse>> {
    // Ownership check: users only see their own products' history.
    repo::products::find_for_user(&state.db, auth.id, id)
        .await?
        .ok_or(AppError::NotFound)?;

    let limit = q.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let (points, stats) = tokio::try_join!(
        repo::prices::history(&state.db, id, limit),
        repo::prices::stats(&state.db, id),
    )?;

    Ok(Json(HistoryResponse {
        product_id: id,
        stats,
        points,
    }))
}
