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
mod telegram;
mod users;

use agents::agent_service::AgentService;
use config::load_config;
use crypto::CryptoService;
use db::Database;
use llm::llm_adapter::LlmAdapter;
use llm::tool_registry::ToolRegistry;
use llm_models::model_service::ModelService;
use llm_providers::provider_service::ProviderService;
use messages::message_service::MessageService;
use routes::{create_router, AppState};
use sessions::session_service::SessionService;
use std::net::SocketAddr;
use std::sync::Arc;
use telegram::telegram_adapter::TelegramAdapter;
use telegram::telegram_registry::TelegramRegistry;
use telegram::telegram_service::TelegramConnectorService;
use telegram::telegram_supervisor::TelegramSupervisor;
use tracing_subscriber;
use users::user_service::UserService;

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

    let crypto = CryptoService::new().expect("Failed to initialize crypto");
    let llm = LlmAdapter::new();
    let tools = ToolRegistry::new();

    let users = UserService::new(db.clone());
    let providers = ProviderService::new(db.clone(), crypto.clone());
    let models = ModelService::new(db.clone());
    let agents = AgentService::new(db.clone());
    let sessions = SessionService::new(db.clone());
    let messages = MessageService::new(db.clone(), llm, crypto.clone());

    let basic_user = users
        .ensure_basic_user()
        .await
        .expect("Failed to seed basic_user");

    tracing::info!("Basic user ensured: {:?}", basic_user.id);

    let telegram_adapter = TelegramAdapter::new();
    let registry = Arc::new(TelegramRegistry::new(db.clone()));
    let telegram_connectors =
        TelegramConnectorService::new(db.clone(), crypto.clone(), telegram_adapter.clone());

    let supervisor = TelegramSupervisor::new(
        registry,
        telegram_adapter,
        telegram_connectors.clone(),
        sessions.clone(),
        messages.clone(),
        basic_user.id,
    );
    supervisor.start();

    tracing::info!("TelegramSupervisor spawned");

    let state = AppState {
        users,
        providers,
        models,
        agents,
        sessions,
        messages,
        telegram_connectors,
        tools,
        basic_user_id: basic_user.id,
    };

    let app = create_router(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");

    tracing::info!(
        "Server started successfully on http://localhost:{}",
        config.server.port
    );

    axum::serve(listener, app)
        .await
        .expect("Server error");
}
