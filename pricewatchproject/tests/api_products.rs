//! API tests.
//!
//! The first group runs without a database: it checks routing, auth and input
//! validation, which all reject a request before any query runs.
//!
//! `full_flow` needs a real Postgres. Set TEST_DATABASE_URL to run it, e.g.
//! TEST_DATABASE_URL=postgres://postgres:postgres@localhost/pricewatch_test cargo test

use std::time::Duration;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
    response::Html,
    routing::get,
};
use http_body_util::BodyExt;
use pricewatchproject::{
    AppState, Config,
    api::{auth::issue_token, router},
};
use serde_json::{Value, json};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use uuid::Uuid;

const SECRET: &str = "integration-test-secret";

fn config(database_url: &str) -> Config {
    Config {
        database_url: database_url.into(),
        jwt_secret: SECRET.into(),
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        check_interval: Duration::from_secs(3600),
        scheduler_tick: Duration::from_secs(60),
        scrape_concurrency: 2,
        notify_webhook_url: None,
    }
}

/// App whose pool never connects; fine for requests rejected before the DB.
fn app_without_db() -> Router {
    let url = "postgres://nobody@127.0.0.1:1/none";
    let db = PgPoolOptions::new()
        .acquire_timeout(Duration::from_millis(200))
        .connect_lazy(url)
        .unwrap();
    router(AppState::new(db, config(url)))
}

async fn send(
    app: &Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let req = match body {
        Some(b) => req
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(b.to_string())),
        None => req.body(Body::empty()),
    }
    .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

#[tokio::test]
async fn health_is_public() {
    let (status, body) = send(&app_without_db(), "GET", "/health", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn products_require_auth() {
    let app = app_without_db();
    let (status, _) = send(&app, "GET", "/api/products", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, body) = send(&app, "GET", "/api/products", Some("not-a-jwt"), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "unauthorized");
}

#[tokio::test]
async fn token_signed_with_other_secret_is_rejected() {
    let token = issue_token(Uuid::new_v4(), "some-other-secret-value").unwrap();
    let (status, _) = send(
        &app_without_db(),
        "GET",
        "/api/products",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn create_validates_input() {
    let app = app_without_db();
    let token = issue_token(Uuid::new_v4(), SECRET).unwrap();

    let cases = [
        json!({ "url": "not a url", "target_price": "10" }),
        json!({ "url": "ftp://shop.example/item", "target_price": "10" }),
        json!({ "url": "https://shop.example/item", "target_price": "0" }),
        json!({ "url": "https://shop.example/item", "target_price": "10", "css_selector": "[[[" }),
    ];
    for body in cases {
        let (status, resp) = send(
            &app,
            "POST",
            "/api/products",
            Some(&token),
            Some(body.clone()),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "body: {body}, response: {resp}"
        );
        assert!(resp["error"].is_string());
    }
}

#[tokio::test]
async fn register_validates_input() {
    let app = app_without_db();
    let (status, _) = send(
        &app,
        "POST",
        "/api/auth/register",
        None,
        Some(json!({ "email": "bad", "password": "longenough" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = send(
        &app,
        "POST",
        "/api/auth/register",
        None,
        Some(json!({ "email": "a@b.com", "password": "short" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn bad_product_id_is_rejected() {
    let token = issue_token(Uuid::new_v4(), SECRET).unwrap();
    let (status, _) = send(
        &app_without_db(),
        "GET",
        "/api/products/123",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// Serves a fake product page and returns its URL.
async fn fake_store() -> String {
    let page = Router::new().route(
        "/item",
        get(|| async {
            Html(
                r#"<html><head><title>Test Kettle</title>
                <script type="application/ld+json">
                {"@type":"Product","name":"Test Kettle",
                 "offers":{"@type":"Offer","price":"79.90","priceCurrency":"EUR"}}
                </script></head><body></body></html>"#,
            )
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, page).await.unwrap() });
    format!("http://{addr}/item")
}

#[tokio::test]
async fn full_flow() {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("skipping full_flow: TEST_DATABASE_URL not set");
        return;
    };
    let db = PgPoolOptions::new().connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&db).await.unwrap();
    let app = router(AppState::new(db, config(&url)));

    // Register and log in.
    let email = format!("user-{}@test.dev", Uuid::new_v4());
    let creds = json!({ "email": email, "password": "correct horse" });
    let (status, _) = send(
        &app,
        "POST",
        "/api/auth/register",
        None,
        Some(creds.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, _) = send(
        &app,
        "POST",
        "/api/auth/register",
        None,
        Some(creds.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, login) = send(&app, "POST", "/api/auth/login", None, Some(creds)).await;
    assert_eq!(status, StatusCode::OK);
    let token = login["token"].as_str().unwrap().to_string();

    // Track a product whose price (79.90) is already below the target.
    let store_url = fake_store().await;
    let (status, product) = send(
        &app,
        "POST",
        "/api/products",
        Some(&token),
        Some(json!({ "url": store_url, "target_price": "80.00" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{product}");
    let id = product["id"].as_str().unwrap();

    // Check now.
    let (status, checked) = send(
        &app,
        "POST",
        &format!("/api/products/{id}/check"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{checked}");
    assert_eq!(checked["price"]["price"], "79.90");
    assert_eq!(checked["product"]["currency"], "EUR");
    assert_eq!(checked["product"]["name"], "Test Kettle");
    assert!(
        checked["product"]["notified_at"].is_string(),
        "alert should be recorded"
    );

    // History.
    let (status, history) = send(
        &app,
        "GET",
        &format!("/api/products/{id}/history"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(history["stats"]["samples"].as_i64().unwrap() >= 1);
    assert_eq!(history["stats"]["min_price"], "79.90");

    // Another user can't see it.
    let stranger = issue_token(Uuid::new_v4(), SECRET).unwrap();
    let (status, _) = send(
        &app,
        "GET",
        &format!("/api/products/{id}"),
        Some(&stranger),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Delete.
    let (status, _) = send(
        &app,
        "DELETE",
        &format!("/api/products/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}
