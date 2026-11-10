pub mod auth;
pub mod history;
pub mod middleware;
pub mod products;

use axum::{
    Json, Router,
    routing::{get, post},
};
use serde_json::{Value, json};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/me", get(auth::me))
        .route("/products", get(products::list).post(products::create))
        .route(
            "/products/{id}",
            get(products::get_one)
                .patch(products::update)
                .delete(products::remove),
        )
        .route("/products/{id}/check", post(products::check_now))
        .route("/products/{id}/history", get(history::history));

    Router::new()
        .route("/health", get(health))
        .nest("/api", api)
        .layer(TraceLayer::new_for_http())
        // Open CORS so a separate frontend can call the API during development.
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
