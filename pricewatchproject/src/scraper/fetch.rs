use std::time::Duration;

use super::ScrapeError;

/// Max page size we are willing to download.
const MAX_BODY_BYTES: usize = 5 * 1024 * 1024;

pub fn build_client() -> reqwest::Client {
    reqwest::Client::builder()
        // Many stores block requests without a browser-like user agent.
        .user_agent(
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 \
             (KHTML, like Gecko) Chrome/130.0 Safari/537.36 PriceWatch/0.1",
        )
        .timeout(Duration::from_secs(20))
        .connect_timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .expect("failed to build HTTP client")
}

/// Only http(s) URLs with a host are accepted.
pub fn validate_url(url: &str) -> Result<reqwest::Url, ScrapeError> {
    let parsed = reqwest::Url::parse(url).map_err(|e| ScrapeError::InvalidUrl(e.to_string()))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(ScrapeError::InvalidUrl(
            "only http and https are supported".into(),
        ));
    }
    if parsed.host_str().is_none() {
        return Err(ScrapeError::InvalidUrl("missing host".into()));
    }
    Ok(parsed)
}

pub async fn fetch_html(client: &reqwest::Client, url: &str) -> Result<String, ScrapeError> {
    let url = validate_url(url)?;
    let resp = client
        .get(url)
        .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml")
        .header(reqwest::header::ACCEPT_LANGUAGE, "en-US,en;q=0.9,ru;q=0.8")
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        return Err(ScrapeError::Status(status.as_u16()));
    }

    let mut body = resp.text().await?;
    if body.len() > MAX_BODY_BYTES {
        let mut cut = MAX_BODY_BYTES;
        while !body.is_char_boundary(cut) {
            cut -= 1;
        }
        body.truncate(cut);
    }
    Ok(body)
}
