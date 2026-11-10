//! Scraping engine: download a product page and extract its price.
//!
//! Detection order (first hit wins):
//! 1. the product's own CSS selector, if the user set one;
//! 2. schema.org JSON-LD (`offers.price`), which most big stores publish;
//! 3. price meta tags (`product:price:amount`, `og:price:amount`, `itemprop=price`).

pub mod fetch;
pub mod generic;
pub mod parse;

use rust_decimal::Decimal;

#[derive(Debug, Clone, PartialEq)]
pub struct ScrapeResult {
    pub price: Decimal,
    pub currency: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ScrapeError {
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
    #[error("request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("store responded with HTTP {0}")]
    Status(u16),
    #[error("invalid CSS selector: {0}")]
    BadSelector(String),
    #[error("no price found on the page (try setting css_selector)")]
    PriceNotFound,
}

pub async fn scrape(
    client: &reqwest::Client,
    url: &str,
    css_selector: Option<&str>,
) -> Result<ScrapeResult, ScrapeError> {
    let html = fetch::fetch_html(client, url).await?;
    // HTML parsing is CPU work and the parsed tree is not Send, so keep it
    // in a sync function off the async path.
    let selector = css_selector.map(str::to_owned);
    tokio::task::spawn_blocking(move || generic::extract(&html, selector.as_deref()))
        .await
        .unwrap_or(Err(ScrapeError::PriceNotFound))
}
