use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::compression::CompressionLayer;
use tracing::{info, warn};

mod api;
mod cache;
mod core;
mod db;
mod models;

use api::{alerts, brief, geo, intelligence, sync, user};
use cache::Cache;
use db::Database;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub cache: Cache,
    pub config: AppConfig,
}

/// Application configuration
#[derive(Clone, Debug)]
pub struct AppConfig {
    pub groq_api_key: String,
    pub port: u16,
    pub database_url: String,
    pub max_alerts_free: i32,
}

impl AppConfig {
    fn from_env() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();
        
        Ok(Self {
            groq_api_key: std::env::var("GROQ_API_KEY")
                .unwrap_or_default(),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:./worldmonitor.db".to_string()),
            max_alerts_free: std::env::var("MAX_ALERTS_FREE")
                .unwrap_or_else(|_| "3".to_string())
                .parse()
                .unwrap_or(3),
        })
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting WorldMonitor Core v0.1.0");

    // Load configuration
    let config = AppConfig::from_env()?;
    info!("Configuration loaded: port={}, db={}", config.port, config.database_url);

    // Initialize database
    let db = Database::new(&config.database_url).await?;
    db.run_migrations().await?;
    info!("Database initialized and migrations applied");

    // Initialize cache
    let cache = Cache::new();
    info!("Cache initialized");

    // Create shared state
    let state = Arc::new(AppState { db, cache, config });

    // Build router with CORS and compression
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // API routes (5 endpoints replacing 60+)
        .route("/api/intelligence", get(intelligence::handler))
        .route("/api/brief", post(brief::handler))
        .route("/api/geo", get(geo::handler))
        .route("/api/alerts", post(alerts::handler))
        .route("/api/sync", get(sync::handler))
        .route("/api/user", get(user::get_handler).post(user::post_handler))
        // Static files (frontend)
        .route("/", get(serve_frontend))
        .route("/*path", get(serve_static))
        .layer(cors)
        .layer(CompressionLayer::new())
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("Server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Serve the main frontend HTML
async fn serve_frontend() -> axum::response::Html<&'static str> {
    axum::response::Html(include_str!("../static/index.html"))
}

/// Serve static files (CSS, JS, assets)
async fn serve_static(axum::extract::Path(path): axum::extract::Path<String>) -> impl axum::response::IntoResponse {
    // For production, serve from static directory
    // In this lean version, everything is embedded
    match path.as_str() {
        "app.js" => axum::response::Html(include_str!("../static/app.js")),
        _ => axum::response::Html("Not found"),
    }
}
