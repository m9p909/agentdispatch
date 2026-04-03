use crate::agents::agent_service::AgentService;
use crate::error::Result;
use crate::llm::tool_registry::ToolRegistry;
use crate::llm_models::model_service::ModelService;
use crate::llm_providers::provider_service::ProviderService;
use crate::messages::message_service::{MessageService, SseEvent};
use crate::sessions::session_service::SessionService;
use crate::telegram::telegram_service::TelegramConnectorService;
use crate::users::user_service::UserService;
use axum::{
    extract::{Path, State},
    http::{Method, Request, Response, StatusCode},
    middleware::Next,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    routing::{delete, get},
    Json, Router,
};
use futures_util::StreamExt;
use serde::Deserialize;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub users: UserService,
    pub providers: ProviderService,
    pub models: ModelService,
    pub agents: AgentService,
    pub sessions: SessionService,
    pub messages: MessageService,
    pub telegram_connectors: TelegramConnectorService,
    pub tools: ToolRegistry,
    pub basic_user_id: Uuid,
}

pub async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let health = state.users.db.health_check().await;
    let status = if health.status == "healthy" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status, Json(health))
}

// ===== Providers =====

#[derive(Deserialize)]
pub struct CreateProviderRequest {
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub api_key: String,
    pub base_url: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateProviderRequest {
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub api_key: String,
    pub base_url: Option<String>,
}

pub async fn providers_list(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let providers = state.providers.list_providers().await?;
    Ok(Json(providers))
}

pub async fn provider_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let provider = state.providers.get_provider_by_id(id).await?;
    Ok(Json(provider))
}

pub async fn providers_create(
    State(state): State<AppState>,
    Json(body): Json<CreateProviderRequest>,
) -> Result<impl IntoResponse> {
    let provider = state
        .providers
        .create_provider(&body.name, &body.r#type, &body.api_key, body.base_url.as_deref())
        .await?;
    Ok((StatusCode::CREATED, Json(provider)))
}

pub async fn providers_update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateProviderRequest>,
) -> Result<impl IntoResponse> {
    let provider = state
        .providers
        .update_provider(id, &body.name, &body.r#type, &body.api_key, body.base_url.as_deref())
        .await?;
    Ok(Json(provider))
}

pub async fn providers_delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    state.providers.delete_provider(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ===== Models =====

#[derive(Deserialize)]
pub struct CreateModelRequest {
    pub provider_id: Uuid,
    pub name: String,
    pub model_identifier: String,
}

#[derive(Deserialize)]
pub struct UpdateModelRequest {
    pub provider_id: Uuid,
    pub name: String,
    pub model_identifier: String,
}

pub async fn models_list(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let models = state.models.list_models().await?;
    Ok(Json(models))
}

pub async fn model_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let model = state.models.get_model_by_id(id).await?;
    Ok(Json(model))
}

pub async fn models_create(
    State(state): State<AppState>,
    Json(body): Json<CreateModelRequest>,
) -> Result<impl IntoResponse> {
    let model = state
        .models
        .create_model(body.provider_id, &body.name, &body.model_identifier)
        .await?;
    Ok((StatusCode::CREATED, Json(model)))
}

pub async fn models_update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateModelRequest>,
) -> Result<impl IntoResponse> {
    let model = state
        .models
        .update_model(id, body.provider_id, &body.name, &body.model_identifier)
        .await?;
    Ok(Json(model))
}

pub async fn models_delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    state.models.delete_model(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ===== Agents =====

#[derive(Deserialize)]
pub struct CreateAgentRequest {
    pub model_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: String,
}

#[derive(Deserialize)]
pub struct UpdateAgentRequest {
    pub model_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: String,
}

pub async fn agents_list(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let agents = state.agents.list_agents(state.basic_user_id, 50, 0).await?;
    Ok(Json(agents))
}

pub async fn agent_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let agent = state.agents.get_agent_by_id(id).await?;
    Ok(Json(agent))
}

pub async fn agents_create(
    State(state): State<AppState>,
    Json(body): Json<CreateAgentRequest>,
) -> Result<impl IntoResponse> {
    let agent = state
        .agents
        .create_agent(
            state.basic_user_id,
            body.model_id,
            &body.name,
            body.description.as_deref(),
            &body.system_prompt,
        )
        .await?;
    Ok((StatusCode::CREATED, Json(agent)))
}

pub async fn agents_update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateAgentRequest>,
) -> Result<impl IntoResponse> {
    let agent = state
        .agents
        .update_agent(
            id,
            body.model_id,
            &body.name,
            body.description.as_deref(),
            &body.system_prompt,
        )
        .await?;
    Ok(Json(agent))
}

pub async fn agents_delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    state.agents.delete_agent(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ===== Sessions =====

#[derive(Deserialize)]
pub struct CreateSessionRequest {
    pub agent_id: Uuid,
    pub title: Option<String>,
}

pub async fn sessions_list(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let sessions = state.sessions.list_sessions(state.basic_user_id).await?;
    Ok(Json(sessions))
}

pub async fn session_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let session = state.sessions.get_session_by_id(id).await?;
    Ok(Json(session))
}

pub async fn sessions_create(
    State(state): State<AppState>,
    Json(body): Json<CreateSessionRequest>,
) -> Result<impl IntoResponse> {
    let session = state
        .sessions
        .create_session(body.agent_id, state.basic_user_id, body.title.as_deref())
        .await?;
    Ok((StatusCode::CREATED, Json(session)))
}

pub async fn sessions_delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    state.sessions.delete_session(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ===== Messages =====

#[derive(Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
}

pub async fn messages_list(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let msgs = state.messages.get_messages_by_session(session_id).await?;
    Ok(Json(msgs))
}

pub async fn messages_create(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(body): Json<SendMessageRequest>,
) -> Result<impl IntoResponse> {
    state
        .messages
        .create_message(session_id, "user", &body.content)
        .await?;
    let msgs = state.messages.get_messages_by_session(session_id).await?;
    Ok(Json(msgs))
}

pub async fn messages_stream(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(body): Json<SendMessageRequest>,
) -> impl IntoResponse {
    if body.content.is_empty() {
        return (StatusCode::BAD_REQUEST, "content required").into_response();
    }

    // Simple ownership check: ensure the session belongs to the basic_user_id
    if let Ok(session) = state.sessions.get_session_by_id(session_id).await {
        if session.user_id != state.basic_user_id {
            return (StatusCode::FORBIDDEN, "forbidden").into_response();
        }
    } else {
        return (StatusCode::NOT_FOUND, "session not found").into_response();
    }

    let stream = state
        .messages
        .stream_response(session_id, body.content, state.tools);
    let sse_stream = stream.map(|res| -> std::result::Result<Event, String> {
        let event = res.unwrap_or_else(|err| SseEvent::Error { message: err.to_string() });
        Ok(Event::default().data(serde_json::to_string(&event).unwrap_or_default()))
    });
    Sse::new(sse_stream)
        .keep_alive(KeepAlive::default())
        .into_response()
}

// ===== Telegram Connectors =====

#[derive(Deserialize)]
pub struct CreateConnectorRequest {
    pub agent_id: Uuid,
    pub bot_token: String,
}

#[derive(Deserialize)]
pub struct SetEnabledRequest {
    pub is_enabled: bool,
}

#[derive(Deserialize)]
pub struct AddWhitelistRequest {
    pub telegram_user_id: i64,
}

pub async fn connectors_list(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let connectors = state.telegram_connectors.list_connectors().await?;
    Ok(Json(connectors))
}

pub async fn connector_get(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let connector = state.telegram_connectors.get_connector(agent_id).await?;
    Ok(Json(connector))
}

pub async fn connectors_create(
    State(state): State<AppState>,
    Json(body): Json<CreateConnectorRequest>,
) -> Result<impl IntoResponse> {
    let connector = state
        .telegram_connectors
        .create_connector(body.agent_id, &body.bot_token)
        .await?;
    Ok((StatusCode::CREATED, Json(connector)))
}

pub async fn connector_set_enabled(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
    Json(body): Json<SetEnabledRequest>,
) -> Result<impl IntoResponse> {
    let connector = state
        .telegram_connectors
        .set_enabled(agent_id, body.is_enabled)
        .await?;
    Ok(Json(connector))
}

pub async fn connectors_delete(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
) -> Result<StatusCode> {
    state.telegram_connectors.delete_connector(agent_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn whitelist_list(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let list = state.telegram_connectors.get_whitelist(agent_id).await?;
    Ok(Json(list))
}

pub async fn whitelist_add(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
    Json(body): Json<AddWhitelistRequest>,
) -> Result<StatusCode> {
    state
        .telegram_connectors
        .add_whitelist_entry(agent_id, body.telegram_user_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn whitelist_remove(
    State(state): State<AppState>,
    Path((agent_id, uid)): Path<(Uuid, i64)>,
) -> Result<StatusCode> {
    state
        .telegram_connectors
        .remove_whitelist_entry(agent_id, uid)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ===== Middleware & Router =====

async fn logging_middleware(
    req: Request<axum::body::Body>,
    next: Next,
) -> Response<axum::body::Body> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    tracing::info!("{} {}", method, uri);
    let response = next.run(req).await;
    tracing::debug!("Response status: {}", response.status());
    response
}

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(Any);

    let api = Router::new()
        .route("/providers", get(providers_list).post(providers_create))
        .route(
            "/providers/:id",
            get(provider_get).put(providers_update).delete(providers_delete),
        )
        .route("/models", get(models_list).post(models_create))
        .route(
            "/models/:id",
            get(model_get).put(models_update).delete(models_delete),
        )
        .route("/agents", get(agents_list).post(agents_create))
        .route(
            "/agents/:id",
            get(agent_get).put(agents_update).delete(agents_delete),
        )
        .route("/sessions", get(sessions_list).post(sessions_create))
        .route(
            "/sessions/:id",
            get(session_get).delete(sessions_delete),
        )
        .route(
            "/sessions/:id/messages",
            get(messages_list).post(messages_create),
        )
        .route("/sessions/:id/stream", axum::routing::post(messages_stream))
        .route(
            "/connectors/telegram",
            get(connectors_list).post(connectors_create),
        )
        .route(
            "/connectors/telegram/:agent_id",
            get(connector_get)
                .patch(connector_set_enabled)
                .delete(connectors_delete),
        )
        .route(
            "/connectors/telegram/:agent_id/whitelist",
            get(whitelist_list).post(whitelist_add),
        )
        .route(
            "/connectors/telegram/:agent_id/whitelist/:uid",
            delete(whitelist_remove),
        );

    Router::new()
        .route("/health", get(health_check))
        .nest("/api/v1", api)
        .layer(axum::middleware::from_fn(logging_middleware))
        .layer(cors)
        .with_state(state)
}
