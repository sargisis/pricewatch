//! Price alerts. Every alert is logged; if `NOTIFY_WEBHOOK_URL` is set it is
//! also POSTed there. The payload has both `text` (Slack) and `content`
//! (Discord) so either works without extra code.

use rust_decimal::Decimal;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct PriceAlert {
    pub product_id: Uuid,
    pub user_email: String,
    pub product_name: Option<String>,
    pub url: String,
    pub price: Decimal,
    pub target_price: Decimal,
    pub currency: Option<String>,
}

impl PriceAlert {
    pub fn message(&self) -> String {
        let name = self.product_name.as_deref().unwrap_or(&self.url);
        let cur = self.currency.as_deref().unwrap_or("");
        format!(
            "Price drop: {name} is now {} {cur} (target {} {cur})\n{}",
            self.price, self.target_price, self.url
        )
        .replace("  ", " ")
    }
}

pub struct Notifier {
    http: reqwest::Client,
    webhook_url: Option<String>,
}

#[derive(Serialize)]
struct WebhookPayload<'a> {
    text: &'a str,
    content: &'a str,
    alert: &'a PriceAlert,
}

impl Notifier {
    pub fn new(http: reqwest::Client, webhook_url: Option<String>) -> Self {
        Self { http, webhook_url }
    }

    pub async fn send(&self, alert: &PriceAlert) -> Result<(), reqwest::Error> {
        let text = alert.message();
        tracing::info!(
            product_id = %alert.product_id,
            user = %alert.user_email,
            price = %alert.price,
            target = %alert.target_price,
            "price alert"
        );

        if let Some(url) = &self.webhook_url {
            self.http
                .post(url)
                .json(&WebhookPayload {
                    text: &text,
                    content: &text,
                    alert,
                })
                .send()
                .await?
                .error_for_status()?;
        }
        Ok(())
    }
}
