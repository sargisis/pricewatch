//! Store-agnostic price extraction from HTML.

use rust_decimal::Decimal;
use scraper::{Html, Selector};
use serde_json::Value;

use super::{
    ScrapeError, ScrapeResult,
    parse::{ParsedPrice, detect_currency, parse_machine_amount, parse_price},
};

pub fn extract(html: &str, css_selector: Option<&str>) -> Result<ScrapeResult, ScrapeError> {
    let doc = Html::parse_document(html);
    let title = page_title(&doc);

    let found = match css_selector {
        Some(sel) => from_selector(&doc, sel)?,
        None => None,
    }
    .or_else(|| from_json_ld(&doc))
    .or_else(|| from_meta(&doc));

    let (price, currency) = found.ok_or(ScrapeError::PriceNotFound)?;
    Ok(ScrapeResult {
        price,
        currency,
        title,
    })
}

type Found = (Decimal, Option<String>);

fn sel(s: &str) -> Selector {
    Selector::parse(s).expect("static selector is valid")
}

fn from_selector(doc: &Html, css: &str) -> Result<Option<Found>, ScrapeError> {
    let selector = Selector::parse(css).map_err(|e| ScrapeError::BadSelector(e.to_string()))?;
    for el in doc.select(&selector) {
        if let Some(p) = element_price(&el) {
            return Ok(Some((p.amount, p.currency)));
        }
    }
    Ok(None)
}

/// Prefer machine-readable attributes, then the visible text.
fn element_price(el: &scraper::ElementRef) -> Option<ParsedPrice> {
    let text: String = el.text().collect();
    for attr in ["content", "data-price", "value"] {
        if let Some(amount) = el.value().attr(attr).and_then(parse_machine_amount) {
            return Some(ParsedPrice {
                amount,
                currency: detect_currency(&text),
            });
        }
    }
    parse_price(&text)
}

fn from_json_ld(doc: &Html) -> Option<Found> {
    doc.select(&sel(r#"script[type="application/ld+json"]"#))
        .filter_map(|el| serde_json::from_str::<Value>(&el.inner_html()).ok())
        .find_map(|json| find_offer_price(&json))
}

/// Walk the JSON-LD tree looking for an `offers` object with a price.
/// Handles arrays, `@graph`, nested `offers`, and `AggregateOffer.lowPrice`.
fn find_offer_price(v: &Value) -> Option<Found> {
    match v {
        Value::Array(items) => items.iter().find_map(find_offer_price),
        Value::Object(map) => {
            if let Some(found) = map.get("offers").and_then(offer_price) {
                return Some(found);
            }
            map.values().find_map(find_offer_price)
        }
        _ => None,
    }
}

fn offer_price(offers: &Value) -> Option<Found> {
    match offers {
        Value::Array(items) => items.iter().find_map(offer_price),
        Value::Object(map) => {
            let price = ["price", "lowPrice"]
                .iter()
                .filter_map(|k| map.get(*k))
                .find_map(json_amount)
                .or_else(|| {
                    // Some stores nest it: offers.priceSpecification.price
                    map.get("priceSpecification")
                        .and_then(offer_price)
                        .map(|f| f.0)
                })?;
            let currency = map
                .get("priceCurrency")
                .and_then(Value::as_str)
                .map(|s| s.trim().to_uppercase())
                .filter(|s| !s.is_empty());
            Some((price, currency))
        }
        _ => None,
    }
}

fn json_amount(v: &Value) -> Option<Decimal> {
    match v {
        Value::Number(n) => parse_machine_amount(&n.to_string()),
        Value::String(s) => parse_machine_amount(s),
        _ => None,
    }
}

fn from_meta(doc: &Html) -> Option<Found> {
    const PRICE_SELECTORS: &[&str] = &[
        r#"meta[property="product:price:amount"]"#,
        r#"meta[property="og:price:amount"]"#,
        r#"meta[itemprop="price"]"#,
        r#"[itemprop="price"]"#,
    ];
    const CURRENCY_SELECTORS: &[&str] = &[
        r#"meta[property="product:price:currency"]"#,
        r#"meta[property="og:price:currency"]"#,
        r#"[itemprop="priceCurrency"]"#,
    ];

    let price = PRICE_SELECTORS
        .iter()
        .find_map(|s| doc.select(&sel(s)).find_map(|el| element_price(&el)))?;

    let currency = CURRENCY_SELECTORS
        .iter()
        .find_map(|s| {
            doc.select(&sel(s)).find_map(|el| {
                el.value()
                    .attr("content")
                    .map(|c| c.trim().to_uppercase())
                    .filter(|c| !c.is_empty())
            })
        })
        .or(price.currency);

    Some((price.amount, currency))
}

fn page_title(doc: &Html) -> Option<String> {
    let og = doc
        .select(&sel(r#"meta[property="og:title"]"#))
        .next()
        .and_then(|el| el.value().attr("content").map(str::to_owned));
    let title = og.or_else(|| {
        doc.select(&sel("title"))
            .next()
            .map(|el| el.text().collect::<String>())
    })?;
    let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
    (!title.is_empty()).then(|| title.chars().take(300).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn json_ld_product() {
        let html = r#"<html><head><title>Phone</title>
            <script type="application/ld+json">
            {"@context":"https://schema.org","@type":"Product","name":"Phone",
             "offers":{"@type":"Offer","price":"499.99","priceCurrency":"usd"}}
            </script></head><body></body></html>"#;
        let r = extract(html, None).unwrap();
        assert_eq!(r.price, d("499.99"));
        assert_eq!(r.currency.as_deref(), Some("USD"));
        assert_eq!(r.title.as_deref(), Some("Phone"));
    }

    #[test]
    fn json_ld_graph_and_aggregate_offer() {
        let html = r#"<script type="application/ld+json">
            {"@graph":[{"@type":"WebPage"},
              {"@type":"Product","offers":{"@type":"AggregateOffer","lowPrice":12990,"priceCurrency":"RUB"}}]}
            </script>"#;
        let r = extract(html, None).unwrap();
        assert_eq!(r.price, d("12990"));
        assert_eq!(r.currency.as_deref(), Some("RUB"));
    }

    #[test]
    fn meta_tags() {
        let html = r#"<meta property="og:title" content="Kettle">
            <meta property="product:price:amount" content="1299.00">
            <meta property="product:price:currency" content="EUR">"#;
        let r = extract(html, None).unwrap();
        assert_eq!(r.price, d("1299.00"));
        assert_eq!(r.currency.as_deref(), Some("EUR"));
        assert_eq!(r.title.as_deref(), Some("Kettle"));
    }

    #[test]
    fn custom_selector_wins() {
        let html = r#"<script type="application/ld+json">{"offers":{"price":"100"}}</script>
            <div class="sale"><span class="now">89 990 ₽</span></div>"#;
        let r = extract(html, Some(".sale .now")).unwrap();
        assert_eq!(r.price, d("89990"));
        assert_eq!(r.currency.as_deref(), Some("RUB"));
    }

    #[test]
    fn falls_back_when_selector_misses() {
        let html = r#"<script type="application/ld+json">{"offers":{"price":"100"}}</script>"#;
        assert_eq!(extract(html, Some(".nope")).unwrap().price, d("100"));
    }

    #[test]
    fn bad_selector_is_an_error() {
        assert!(matches!(
            extract("<p></p>", Some("[[[")),
            Err(ScrapeError::BadSelector(_))
        ));
    }

    #[test]
    fn machine_values_use_dot_decimal() {
        let html = r#"<script type="application/ld+json">{"offers":{"price":"1299.000"}}</script>"#;
        assert_eq!(extract(html, None).unwrap().price, d("1299"));
    }

    #[test]
    fn itemprop_text_with_symbol() {
        let html = r#"<span itemprop="price">1.299,95 €</span>"#;
        let r = extract(html, None).unwrap();
        assert_eq!(r.price, d("1299.95"));
        assert_eq!(r.currency.as_deref(), Some("EUR"));
    }

    #[test]
    fn no_price() {
        assert!(matches!(
            extract("<html><body>hello</body></html>", None),
            Err(ScrapeError::PriceNotFound)
        ));
    }
}
