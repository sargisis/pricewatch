use std::sync::Arc;

use sqlx::PgPool;

use crate::{config::Config, notify::Notifier, scraper};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
    pub http: reqwest::Client,
    pub notifier: Arc<Notifier>,
}

impl AppState {
    pub fn new(db: PgPool, config: Config) -> Self {
        let http = scraper::fetch::build_client();
        let notifier = Arc::new(Notifier::new(
            http.clone(),
            config.notify_webhook_url.clone(),
        ));
        Self {
            db,
            config: Arc::new(config),
            http,
            notifier,
        }
    }
}
