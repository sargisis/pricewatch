use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use rust_decimal::Decimal;
use serde::Serialize;
use uuid::Uuid;

use crate::{
    api::middleware::AuthUser,
    domain::{
        price::PricePoint,
        product::{NewProduct, Product, ProductPatch},
    },
    error::{AppError, AppResult},
    repo,
    scheduler::job,
    scraper::fetch::validate_url,
    state::AppState,
};

fn validate_target(price: Decimal) -> AppResult<()> {
    if price <= Decimal::ZERO {
        return Err(AppError::BadRequest(
            "target_price must be greater than 0".into(),
        ));
    }
    Ok(())
}

fn validate_selector(selector: Option<&str>) -> AppResult<()> {
    if let Some(s) = selector {
        ::scraper::Selector::parse(s)
            .map_err(|e| AppError::BadRequest(format!("invalid css_selector: {e}")))?;
    }
    Ok(())
}

/// Treat "" the same as "not set".
fn non_empty(v: Option<String>) -> Option<String> {
    v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

pub async fn list(State(state): State<AppState>, auth: AuthUser) -> AppResult<Json<Vec<Product>>> {
    Ok(Json(
        repo::products::list_for_user(&state.db, auth.id).await?,
    ))
}

pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(mut body): Json<NewProduct>,
) -> AppResult<(StatusCode, Json<Product>)> {
    body.url = validate_url(body.url.trim())
        .map_err(|e| AppError::BadRequest(e.to_string()))?
        .to_string();
    body.name = non_empty(body.name);
    body.css_selector = non_empty(body.css_selector);
    validate_target(body.target_price)?;
    validate_selector(body.css_selector.as_deref())?;

    let product = repo::products::create(&state.db, auth.id, &body)
        .await
        .map_err(|e| match e.as_database_error() {
            Some(db) if db.is_unique_violation() => {
                AppError::Conflict("you are already tracking this URL".into())
            }
            _ => AppError::Db(e),
        })?;

    // Get the first price right away instead of waiting for the scheduler.
    let bg_state = state.clone();
    let bg_product = product.clone();
    tokio::spawn(async move {
        if let Err(e) = job::check_product(&bg_state, &bg_product).await {
            tracing::warn!(product_id = %bg_product.id, error = %e, "initial check failed");
        }
    });

    Ok((StatusCode::CREATED, Json(product)))
}

pub async fn get_one(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Product>> {
    repo::products::find_for_user(&state.db, auth.id, id)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound)
}

pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(mut patch): Json<ProductPatch>,
) -> AppResult<Json<Product>> {
    if let Some(price) = patch.target_price {
        validate_target(price)?;
    }
    patch.name = non_empty(patch.name);
    patch.css_selector = non_empty(patch.css_selector);
    validate_selector(patch.css_selector.as_deref())?;

    repo::products::update(&state.db, auth.id, id, &patch)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound)
}

pub async fn remove(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    if repo::products::delete(&state.db, auth.id, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}

#[derive(Serialize)]
pub struct CheckResponse {
    pub price: PricePoint,
    pub product: Product,
}

/// Scrape the product now and return the new price.
pub async fn check_now(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<CheckResponse>> {
    let product = repo::products::find_for_user(&state.db, auth.id, id)
        .await?
        .ok_or(AppError::NotFound)?;

    let price = job::check_product(&state, &product).await?;
    let product = repo::products::find_for_user(&state.db, auth.id, id)
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(Json(CheckResponse { price, product }))
}
