use std::{env, net::SocketAddr, time::Duration};

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub bind_addr: SocketAddr,
    /// How long to wait before re-checking the same product.
    pub check_interval: Duration,
    /// How often the scheduler wakes up to look for products that are due.
    pub scheduler_tick: Duration,
    /// Max number of pages scraped at the same time.
    pub scrape_concurrency: usize,
    /// Optional Slack/Discord-compatible webhook for price alerts.
    pub notify_webhook_url: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("missing required environment variable {0}")]
    Missing(&'static str),
    #[error("invalid value for {0}: {1}")]
    Invalid(&'static str, String),
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let jwt_secret = required("JWT_SECRET")?;
        if jwt_secret.len() < 16 {
            return Err(ConfigError::Invalid(
                "JWT_SECRET",
                "must be at least 16 characters".into(),
            ));
        }

        Ok(Self {
            database_url: required("DATABASE_URL")?,
            jwt_secret,
            bind_addr: bind_addr()?,
            check_interval: Duration::from_secs(parsed("CHECK_INTERVAL_SECS", "3600")?),
            scheduler_tick: Duration::from_secs(parsed("SCHEDULER_TICK_SECS", "60")?),
            scrape_concurrency: parsed::<usize>("SCRAPE_CONCURRENCY", "4")?.max(1),
            notify_webhook_url: env::var("NOTIFY_WEBHOOK_URL")
                .ok()
                .filter(|s| !s.is_empty()),
        })
    }
}

/// `BIND_ADDR` wins. Otherwise use `PORT`, which hosting platforms such as
/// Render set for the container. Falls back to port 3000.
fn bind_addr() -> Result<SocketAddr, ConfigError> {
    if env::var("BIND_ADDR").is_ok_and(|v| !v.is_empty()) {
        return parsed("BIND_ADDR", "0.0.0.0:3000");
    }
    let port: u16 = parsed("PORT", "3000")?;
    Ok(SocketAddr::from(([0, 0, 0, 0], port)))
}

fn required(key: &'static str) -> Result<String, ConfigError> {
    env::var(key)
        .ok()
        .filter(|v| !v.is_empty())
        .ok_or(ConfigError::Missing(key))
}

fn parsed<T: std::str::FromStr>(key: &'static str, default: &str) -> Result<T, ConfigError>
where
    T::Err: std::fmt::Display,
{
    let raw = env::var(key).unwrap_or_else(|_| default.to_string());
    raw.parse()
        .map_err(|e: T::Err| ConfigError::Invalid(key, e.to_string()))
}
