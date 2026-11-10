//! Background scheduler: every tick, find products that are due for a check
//! and scrape them with bounded concurrency.

pub mod job;

use std::sync::Arc;

use tokio::{
    sync::{Semaphore, watch},
    task::JoinSet,
};

use crate::{repo, state::AppState};

/// How many due products we pick up per tick.
const BATCH_SIZE: i64 = 100;

/// Runs until `shutdown` receives `true`.
pub fn spawn(state: AppState, mut shutdown: watch::Receiver<bool>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(state.config.scheduler_tick);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        tracing::info!(
            tick = ?state.config.scheduler_tick,
            interval = ?state.config.check_interval,
            "scheduler started"
        );

        loop {
            tokio::select! {
                _ = shutdown.wait_for(|stop| *stop) => break,
                _ = ticker.tick() => {}
            }
            if let Err(e) = run_once(&state).await {
                tracing::error!(error = %e, "scheduler tick failed");
            }
        }
        tracing::info!("scheduler stopped");
    })
}

/// One pass: check every product that is due. Returns how many were checked.
pub async fn run_once(state: &AppState) -> sqlx::Result<usize> {
    let due =
        repo::products::due_for_check(&state.db, state.config.check_interval, BATCH_SIZE).await?;
    if due.is_empty() {
        return Ok(0);
    }
    tracing::debug!(count = due.len(), "checking due products");

    let limit = Arc::new(Semaphore::new(state.config.scrape_concurrency));
    let mut tasks = JoinSet::new();
    let count = due.len();

    for product in due {
        let state = state.clone();
        let limit = limit.clone();
        tasks.spawn(async move {
            let _permit = limit.acquire_owned().await.expect("semaphore closed");
            if let Err(e) = job::check_product(&state, &product).await {
                tracing::warn!(product_id = %product.id, url = %product.url, error = %e, "check failed");
            }
        });
    }
    while tasks.join_next().await.is_some() {}
    Ok(count)
}
