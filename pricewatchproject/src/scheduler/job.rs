//! Checking a single product. Used by the scheduler and by the
//! "check now" API endpoint.

use crate::{
    domain::{price::PricePoint, product::Product},
    error::{AppError, AppResult},
    notify::PriceAlert,
    repo, scraper,
    state::AppState,
};

/// Scrape the product page, store the price, and send an alert if it reached
/// the target. Scrape failures are recorded on the product and returned.
pub async fn check_product(state: &AppState, product: &Product) -> AppResult<PricePoint> {
    let result =
        match scraper::scrape(&state.http, &product.url, product.css_selector.as_deref()).await {
            Ok(r) => r,
            Err(e) => {
                repo::products::record_failure(&state.db, product.id, &e.to_string()).await?;
                return Err(e.into());
            }
        };

    let currency = result.currency.as_deref().or(product.currency.as_deref());
    let point = repo::prices::insert(&state.db, product.id, result.price, currency).await?;
    repo::products::record_success(
        &state.db,
        product.id,
        result.price,
        result.currency.as_deref(),
        result.title.as_deref(),
    )
    .await?;

    tracing::info!(product_id = %product.id, price = %result.price, "price recorded");

    if result.price <= product.target_price {
        // mark_notified is atomic, so concurrent checks can't double-alert.
        if repo::products::mark_notified(&state.db, product.id).await? {
            let user = repo::users::find_by_id(&state.db, product.user_id)
                .await?
                .ok_or_else(|| AppError::Internal("product owner not found".into()))?;
            let alert = PriceAlert {
                product_id: product.id,
                user_email: user.email,
                product_name: product.name.clone().or(result.title.clone()),
                url: product.url.clone(),
                price: result.price,
                target_price: product.target_price,
                currency: currency.map(str::to_owned),
            };
            if let Err(e) = state.notifier.send(&alert).await {
                // Let the next check retry the alert.
                tracing::error!(product_id = %product.id, error = %e, "failed to send alert");
                repo::products::clear_notified(&state.db, product.id).await?;
            }
        }
    } else if product.notified_at.is_some() {
        // Price went back up: re-arm so the next drop alerts again.
        repo::products::clear_notified(&state.db, product.id).await?;
    }

    Ok(point)
}
