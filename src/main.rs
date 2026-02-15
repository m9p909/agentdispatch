mod agents;
mod config;
mod crypto;
mod db;
mod error;
mod handler;
mod llm;
mod llm_models;
mod llm_providers;
mod messages;
mod routes;
mod schema;
mod sessions;
mod users;
pub mod views;

use config::load_config;
use db::Database;
use routes::{create_router, AppState};
use std::net::SocketAddr;
use tracing_subscriber;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(true)
        .with_level(true)
        .init();

    tracing::info!("Starting Agent Builder...");

    let config = load_config().expect("Failed to load config");
    tracing::info!(
        "Server config: {}:{}",
        config.server.host,
        config.server.port
    );

    let db = Database::new(&config.db)
        .await
        .expect("Failed to connect to database");

    tracing::info!("Database connected successfully");

    schema::create_schema(db.get_pool())
        .await
        .expect("Failed to create schema");

    tracing::info!("Database schema initialized");

    let basic_user = users::service::ensure_basic_user(db.get_pool())
        .await
        .expect("Failed to seed basic_user");

    tracing::info!("Basic user ensured: {:?}", basic_user.id);

    let state = AppState {
        db,
        basic_user_id: basic_user.id,
    };

    let app = create_router(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");

    tracing::info!("Server started successfully on http://localhost:{}", config.server.port);

    axum::serve(listener, app)
        .await
        .expect("Server error");
}
