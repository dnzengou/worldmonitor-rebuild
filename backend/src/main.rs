//! WorldMonitor Agents - Chain-of-Thought Multi-Agent Intelligence System
//!
//! A lean, edge-native OSINT platform with:
//! - Multi-agent architecture with chain-of-thought reasoning
//! - 5 specialized agents replacing 60+ edge functions
//! - Real-time intelligence fusion from GDELT + RSS
//! - AI-powered brief generation
//! - User engagement features (streaks, notifications)
//! - Monetization (Freemium model)

use axum::{
    extract::{Query, State, Path},
    http::StatusCode,
    response::{Json, IntoResponse},
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::compression::CompressionLayer;
use tracing::{info, error};

mod agents;
mod api;
mod config;
mod db;
mod models;
mod services;

use agents::{AgentCoordinator, AgentTask, AgentWorkflow, TaskType, WorkflowStep, TaskPriority};
use services::{cache::CacheService, llm::LlmService, payments::PaymentService};

/// Application state shared across handlers
pub struct AppState {
    pub coordinator: Arc<AgentCoordinator>,
    pub db: Arc<db::Database>,
    pub cache: Arc<CacheService>,
    pub config: Arc<config::Config>,
    pub payments: Arc<PaymentService>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("🚀 Starting WorldMonitor Agents v0.2.0");

    // Load configuration
    let config = Arc::new(config::Config::from_env()?);
    info!("Configuration loaded");

    // Initialize database
    let db = Arc::new(db::Database::new(&config.database_url).await?);
    db.run_migrations().await?;
    info!("Database initialized");

    // Initialize cache
    let cache = Arc::new(CacheService::new());
    info!("Cache initialized");

    // Initialize LLM service
    let llm = Arc::new(LlmService::new(config.groq_api_key.clone()));
    info!("LLM service initialized");

    // Initialize payment service
    let payments = Arc::new(PaymentService::new(config.stripe_secret_key.clone()));
    info!("Payment service initialized");

    // Create agent context
    let agent_context = Arc::new(agents::AgentContext::new(
        db.clone(),
        cache.clone(),
        config.clone(),
        llm.clone(),
    ));

    // Initialize agent coordinator
    let coordinator = Arc::new(AgentCoordinator::new(agent_context));
    coordinator.initialize().await?;
    info!("Agent coordinator initialized with 5 agents");

    // Create shared state
    let state = Arc::new(AppState {
        coordinator,
        db,
        cache,
        config,
        payments,
    });

    // Build router
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // API routes
        .route("/api/intelligence", get(api::intelligence::handler))
        .route("/api/brief", post(api::brief::handler))
        .route("/api/geo", get(api::geo::handler))
        .route("/api/sync", get(api::sync::handler))
        .route("/api/alerts", post(api::alerts::create_handler))
        .route("/api/alerts", get(api::alerts::list_handler))
        .route("/api/user", get(api::user::get_handler))
        .route("/api/user", post(api::user::update_handler))
        .route("/api/agents/status", get(api::agents::status_handler))
        .route("/api/pricing", get(api::payments::pricing_handler))
        .route("/api/subscribe", post(api::payments::subscribe_handler))
        // Static files
        .route("/", get(serve_frontend))
        .route("/*path", get(serve_static))
        .layer(cors)
        .layer(CompressionLayer::new())
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("🌐 Server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn serve_frontend() -> axum::response::Html<&'static str> {
    axum::response::Html(include_str!("../../frontend/dist/index.html"))
}

async fn serve_static(Path(path): Path<String>) -> impl IntoResponse {
    match path.as_str() {
        "app.js" => ([("content-type", "application/javascript")], 
            include_str!("../../frontend/dist/app.js")),
        "styles.css" => ([("content-type", "text/css")], 
            include_str!("../../frontend/dist/styles.css")),
        _ => ([("content-type", "text/plain")], "Not found"),
    }
}
