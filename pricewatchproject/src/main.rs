use std::{path::PathBuf, time::Duration};

use pricewatchproject::{AppState, Config, api::router, scheduler};
use sqlx::postgres::PgPoolOptions;
use tokio::{net::TcpListener, sync::watch};
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pricewatchproject=info,tower_http=info".into()),
        )
        .init();

    let config = Config::from_env()?;

    let db = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&db).await?;
    tracing::info!("database ready");

    let bind_addr = config.bind_addr;
    let state = AppState::new(db, config);

    let (stop_tx, stop_rx) = watch::channel(false);
    let scheduler = scheduler::spawn(state.clone(), stop_rx);

    // Serve the built frontend (frontend/dist) from the same address, if present.
    let static_dir =
        PathBuf::from(std::env::var("STATIC_DIR").unwrap_or_else(|_| "../frontend/dist".into()));
    let mut app = router(state);
    if static_dir.join("index.html").is_file() {
        tracing::info!("serving frontend from {}", static_dir.display());
        app = app.fallback_service(
            ServeDir::new(&static_dir).fallback(ServeFile::new(static_dir.join("index.html"))),
        );
    }

    let listener = TcpListener::bind(bind_addr).await?;
    tracing::info!("listening on http://{bind_addr}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    stop_tx.send(true).ok();
    scheduler.await.ok();
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.ok();
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutting down");
}
