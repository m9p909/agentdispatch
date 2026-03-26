use crate::db::Database;
use crate::error::Result;
use crate::{agents, llm_models, llm_providers, messages, sessions};
use axum::{
    extract::{Path, State},
    http::{Method, Request, Response, StatusCode},
    middleware::Next,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub basic_user_id: Uuid,
}

pub async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let health = state.db.health_check().await;
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
    let providers = llm_providers::service::list_providers(state.db.get_pool()).await?;
    Ok(Json(providers))
}

pub async fn provider_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let provider = llm_providers::service::get_provider_by_id(state.db.get_pool(), id).await?;
    Ok(Json(provider))
}

pub async fn providers_create(
    State(state): State<AppState>,
    Json(body): Json<CreateProviderRequest>,
) -> Result<impl IntoResponse> {
    let provider = llm_providers::service::create_provider(
        state.db.get_pool(),
        &body.name,
        &body.r#type,
        &body.api_key,
        body.base_url.as_deref(),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(provider)))
}

pub async fn providers_update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateProviderRequest>,
) -> Result<impl IntoResponse> {
    let provider = llm_providers::service::update_provider(
        state.db.get_pool(),
        id,
        &body.name,
        &body.r#type,
        &body.api_key,
        body.base_url.as_deref(),
    )
    .await?;
    Ok(Json(provider))
}

pub async fn providers_delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    llm_providers::service::delete_provider(state.db.get_pool(), id).await?;
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
    let models = llm_models::service::list_models(state.db.get_pool()).await?;
    Ok(Json(models))
}

pub async fn model_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let model = llm_models::service::get_model_by_id(state.db.get_pool(), id).await?;
    Ok(Json(model))
}

pub async fn models_create(
    State(state): State<AppState>,
    Json(body): Json<CreateModelRequest>,
) -> Result<impl IntoResponse> {
    let model = llm_models::service::create_model(
        state.db.get_pool(),
        body.provider_id,
        &body.name,
        &body.model_identifier,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(model)))
}

pub async fn models_update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateModelRequest>,
) -> Result<impl IntoResponse> {
    let model = llm_models::service::update_model(
        state.db.get_pool(),
        id,
        body.provider_id,
        &body.name,
        &body.model_identifier,
    )
    .await?;
    Ok(Json(model))
}

pub async fn models_delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    llm_models::service::delete_model(state.db.get_pool(), id).await?;
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
    let agents =
        agents::service::list_agents(state.db.get_pool(), state.basic_user_id, 50, 0).await?;
    Ok(Json(agents))
}

pub async fn agent_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let agent = agents::service::get_agent_by_id(state.db.get_pool(), id).await?;
    Ok(Json(agent))
}

pub async fn agents_create(
    State(state): State<AppState>,
    Json(body): Json<CreateAgentRequest>,
) -> Result<impl IntoResponse> {
    let agent = agents::service::create_agent(
        state.db.get_pool(),
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
    let agent = agents::service::update_agent(
        state.db.get_pool(),
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
    agents::service::delete_agent(state.db.get_pool(), id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ===== Sessions =====

#[derive(Deserialize)]
pub struct CreateSessionRequest {
    pub agent_id: Uuid,
    pub title: Option<String>,
}

pub async fn sessions_list(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let sessions =
        sessions::service::list_sessions(state.db.get_pool(), state.basic_user_id).await?;
    Ok(Json(sessions))
}

pub async fn session_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let session = sessions::service::get_session_by_id(state.db.get_pool(), id).await?;
    Ok(Json(session))
}

pub async fn sessions_create(
    State(state): State<AppState>,
    Json(body): Json<CreateSessionRequest>,
) -> Result<impl IntoResponse> {
    let session = sessions::service::create_session(
        state.db.get_pool(),
        body.agent_id,
        state.basic_user_id,
        body.title.as_deref(),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(session)))
}

pub async fn sessions_delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    sessions::service::delete_session(state.db.get_pool(), id).await?;
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
    let msgs =
        messages::service::get_messages_by_session(state.db.get_pool(), session_id).await?;
    Ok(Json(msgs))
}

pub async fn messages_create(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(body): Json<SendMessageRequest>,
) -> Result<impl IntoResponse> {
    messages::service::create_message(state.db.get_pool(), session_id, "user", &body.content)
        .await?;
    let msgs =
        messages::service::get_messages_by_session(state.db.get_pool(), session_id).await?;
    Ok(Json(msgs))
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
        );

    Router::new()
        .route("/health", get(health_check))
        .nest("/api/v1", api)
        .layer(axum::middleware::from_fn(logging_middleware))
        .layer(cors)
        .with_state(state)
}
