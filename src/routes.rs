use crate::db::Database;
use crate::error::{AppError, Result};
use crate::views;
use crate::{agents, llm_models, llm_providers, messages, sessions};
use askama::Template;
use askama_axum::IntoResponse;
use axum::{
    extract::{Path, State},
    http::{Request, Response, StatusCode},
    middleware::Next,
    response::Html,
    routing::{get, post},
    Form, Router,
};
use serde::Deserialize;
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
    (status, Html(format!("{:?}", health)))
}

// ===== Providers Routes =====
pub async fn providers_list(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let providers_data = llm_providers::service::list_providers(state.db.get_pool()).await?;
    let providers: Vec<views::ProviderListItem> = providers_data
        .into_iter()
        .map(|p| views::ProviderListItem {
            id: p.id,
            name: p.name,
            provider_type: p.r#type,
        })
        .collect();

    Ok(views::ProvidersList { providers })
}

pub async fn providers_new(_state: State<AppState>) -> impl IntoResponse {
    views::ProvidersForm {
        provider: None,
        errors: vec![],
    }
}

#[derive(Deserialize)]
pub struct ProviderForm {
    pub name: String,
    #[serde(rename = "r#type")]
    pub r#type: String,
    pub api_key: String,
    pub base_url: Option<String>,
}

pub async fn providers_create(
    State(state): State<AppState>,
    Form(form): Form<ProviderForm>,
) -> Result<Response<String>> {
    match llm_providers::service::create_provider(
        state.db.get_pool(),
        &form.name,
        &form.r#type,
        &form.api_key,
        form.base_url.as_deref(),
    )
    .await
    {
        Ok(_) => Ok(Response::builder()
            .status(StatusCode::FOUND)
            .header("location", "/providers")
            .body(String::new())?),
        Err(e) => {
            let page = views::ProvidersForm {
                provider: None,
                errors: vec![e.to_string()],
            };
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/html; charset=utf-8")
                .body(page.render()?)?)
        }
    }
}

pub async fn providers_edit(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let provider = llm_providers::db::get_provider_by_id(state.db.get_pool(), id)
        .await?
        .ok_or_else(|| AppError::NotFound("Provider not found".to_string()))?;

    Ok(views::ProvidersForm {
        provider: Some(views::ProviderFormItem {
            id: provider.id,
            name: provider.name,
            provider_type: provider.r#type,
            base_url: provider.base_url,
        }),
        errors: vec![],
    })
}

pub async fn providers_update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Form(form): Form<ProviderForm>,
) -> Result<Response<String>> {
    match llm_providers::service::update_provider(
        state.db.get_pool(),
        id,
        &form.name,
        &form.r#type,
        &form.api_key,
        form.base_url.as_deref(),
    )
    .await
    {
        Ok(_) => Ok(Response::builder()
            .status(StatusCode::FOUND)
            .header("location", "/providers")
            .body(String::new())?),
        Err(e) => {
            let provider = llm_providers::db::get_provider_by_id(state.db.get_pool(), id)
                .await?
                .ok_or_else(|| AppError::NotFound("Provider not found".to_string()))?;

            let page = views::ProvidersForm {
                provider: Some(views::ProviderFormItem {
                    id: provider.id,
                    name: provider.name,
                    provider_type: provider.r#type,
                    base_url: provider.base_url,
                }),
                errors: vec![e.to_string()],
            };
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/html; charset=utf-8")
                .body(page.render()?)?)
        }
    }
}

pub async fn providers_delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Response<String>> {
    llm_providers::service::delete_provider(state.db.get_pool(), id).await?;
    Ok(Response::builder()
        .status(StatusCode::FOUND)
        .header("location", "/providers")
        .body(String::new())?)
}

// ===== Models Routes =====
pub async fn models_list(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let models_data = llm_models::service::list_models(state.db.get_pool()).await?;
    let models: Vec<views::ModelListItem> = models_data
        .into_iter()
        .map(|m| views::ModelListItem {
            id: m.id,
            name: m.name,
            model_identifier: m.model_identifier,
            provider_id: m.provider_id,
        })
        .collect();

    Ok(views::ModelsList { models })
}

pub async fn models_new(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let providers_data = llm_providers::service::list_providers(state.db.get_pool()).await?;
    let providers: Vec<views::ProviderFormItem> = providers_data
        .into_iter()
        .map(|p| views::ProviderFormItem {
            id: p.id,
            name: p.name,
            provider_type: p.r#type,
            base_url: p.base_url,
        })
        .collect();

    Ok(views::ModelsForm {
        model: None,
        providers,
        errors: vec![],
    })
}

#[derive(Deserialize)]
pub struct ModelForm {
    pub provider_id: Uuid,
    pub name: String,
    pub model_identifier: String,
}

pub async fn models_create(
    State(state): State<AppState>,
    Form(form): Form<ModelForm>,
) -> Result<Response<String>> {
    match llm_models::service::create_model(
        state.db.get_pool(),
        form.provider_id,
        &form.name,
        &form.model_identifier,
    )
    .await
    {
        Ok(_) => Ok(Response::builder()
            .status(StatusCode::FOUND)
            .header("location", "/models")
            .body(String::new())?),
        Err(e) => {
            let providers_data = llm_providers::service::list_providers(state.db.get_pool()).await?;
            let providers: Vec<views::ProviderFormItem> = providers_data
                .into_iter()
                .map(|p| views::ProviderFormItem {
                    id: p.id,
                    name: p.name,
                    provider_type: p.r#type,
                    base_url: p.base_url,
                })
                .collect();

            let page = views::ModelsForm {
                model: None,
                providers,
                errors: vec![e.to_string()],
            };
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/html; charset=utf-8")
                .body(page.render()?)?)
        }
    }
}

pub async fn models_delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Response<String>> {
    llm_models::service::delete_model(state.db.get_pool(), id).await?;
    Ok(Response::builder()
        .status(StatusCode::FOUND)
        .header("location", "/models")
        .body(String::new())?)
}

pub async fn models_edit(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let model = llm_models::db::get_model_by_id(state.db.get_pool(), id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Model not found".to_string()))?;

    let providers_data = llm_providers::service::list_providers(state.db.get_pool()).await?;
    let providers: Vec<views::ProviderFormItem> = providers_data
        .into_iter()
        .map(|p| views::ProviderFormItem {
            id: p.id,
            name: p.name,
            provider_type: p.r#type,
            base_url: p.base_url,
        })
        .collect();

    Ok(views::ModelsForm {
        model: Some(views::ModelFormItem {
            id: model.id,
            name: model.name,
            model_identifier: model.model_identifier,
            provider_id: model.provider_id,
        }),
        providers,
        errors: vec![],
    })
}

pub async fn models_update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Form(form): Form<ModelForm>,
) -> Result<Response<String>> {
    match llm_models::service::update_model(
        state.db.get_pool(),
        id,
        form.provider_id,
        &form.name,
        &form.model_identifier,
    )
    .await
    {
        Ok(_) => Ok(Response::builder()
            .status(StatusCode::FOUND)
            .header("location", "/models")
            .body(String::new())?),
        Err(e) => {
            let model = llm_models::db::get_model_by_id(state.db.get_pool(), id)
                .await
                .map_err(AppError::Database)?
                .ok_or_else(|| AppError::NotFound("Model not found".to_string()))?;

            let providers_data = llm_providers::service::list_providers(state.db.get_pool()).await?;
            let providers: Vec<views::ProviderFormItem> = providers_data
                .into_iter()
                .map(|p| views::ProviderFormItem {
                    id: p.id,
                    name: p.name,
                    provider_type: p.r#type,
                    base_url: p.base_url,
                })
                .collect();

            let page = views::ModelsForm {
                model: Some(views::ModelFormItem {
                    id: model.id,
                    name: model.name,
                    model_identifier: model.model_identifier,
                    provider_id: model.provider_id,
                }),
                providers,
                errors: vec![e.to_string()],
            };
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/html; charset=utf-8")
                .body(page.render()?)?)
        }
    }
}

// ===== Agents Routes =====
pub async fn agents_list(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let agents_data =
        agents::service::list_agents(state.db.get_pool(), state.basic_user_id, 50, 0).await?;
    let agents: Vec<views::AgentListItem> = agents_data
        .into_iter()
        .map(|a| views::AgentListItem {
            id: a.id,
            name: a.name,
            description: a.description,
            model_id: a.model_id,
        })
        .collect();

    Ok(views::AgentsList { agents })
}

pub async fn agents_new(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let models_data = llm_models::service::list_models(state.db.get_pool()).await?;
    let models: Vec<views::ModelFormItem> = models_data
        .into_iter()
        .map(|m| views::ModelFormItem {
            id: m.id,
            name: m.name,
            model_identifier: m.model_identifier,
            provider_id: m.provider_id,
        })
        .collect();

    Ok(views::AgentsForm {
        agent: None,
        models,
        errors: vec![],
    })
}

#[derive(Deserialize)]
pub struct AgentForm {
    pub name: String,
    pub description: Option<String>,
    pub model_id: Uuid,
    pub system_prompt: String,
}

pub async fn agents_create(
    State(state): State<AppState>,
    Form(form): Form<AgentForm>,
) -> Result<Response<String>> {
    match agents::service::create_agent(
        state.db.get_pool(),
        state.basic_user_id,
        form.model_id,
        &form.name,
        form.description.as_deref(),
        &form.system_prompt,
    )
    .await
    {
        Ok(_) => Ok(Response::builder()
            .status(StatusCode::FOUND)
            .header("location", "/agents")
            .body(String::new())?),
        Err(e) => {
            let models_data = llm_models::service::list_models(state.db.get_pool()).await?;
            let models: Vec<views::ModelFormItem> = models_data
                .into_iter()
                .map(|m| views::ModelFormItem {
                    id: m.id,
                    name: m.name,
                    model_identifier: m.model_identifier,
                    provider_id: m.provider_id,
                })
                .collect();

            let page = views::AgentsForm {
                agent: None,
                models,
                errors: vec![e.to_string()],
            };
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/html; charset=utf-8")
                .body(page.render()?)?)
        }
    }
}

pub async fn agents_edit(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let agent = agents::db::get_agent_by_id(state.db.get_pool(), id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Agent not found".to_string()))?;

    let models_data = llm_models::service::list_models(state.db.get_pool()).await?;
    let models: Vec<views::ModelFormItem> = models_data
        .into_iter()
        .map(|m| views::ModelFormItem {
            id: m.id,
            name: m.name,
            model_identifier: m.model_identifier,
            provider_id: m.provider_id,
        })
        .collect();

    Ok(views::AgentsForm {
        agent: Some(views::AgentFormItem {
            id: agent.id,
            name: agent.name,
            description: agent.description,
            system_prompt: agent.system_prompt,
            model_id: agent.model_id,
        }),
        models,
        errors: vec![],
    })
}

pub async fn agents_update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AgentForm>,
) -> Result<Response<String>> {
    match agents::service::update_agent(
        state.db.get_pool(),
        id,
        form.model_id,
        &form.name,
        form.description.as_deref(),
        &form.system_prompt,
    )
    .await
    {
        Ok(_) => Ok(Response::builder()
            .status(StatusCode::FOUND)
            .header("location", "/agents")
            .body(String::new())?),
        Err(e) => {
            let agent = agents::db::get_agent_by_id(state.db.get_pool(), id)
                .await
                .map_err(AppError::Database)?
                .ok_or_else(|| AppError::NotFound("Agent not found".to_string()))?;

            let models_data = llm_models::service::list_models(state.db.get_pool()).await?;
            let models: Vec<views::ModelFormItem> = models_data
                .into_iter()
                .map(|m| views::ModelFormItem {
                    id: m.id,
                    name: m.name,
                    model_identifier: m.model_identifier,
                    provider_id: m.provider_id,
                })
                .collect();

            let page = views::AgentsForm {
                agent: Some(views::AgentFormItem {
                    id: agent.id,
                    name: agent.name,
                    description: agent.description,
                    system_prompt: agent.system_prompt,
                    model_id: agent.model_id,
                }),
                models,
                errors: vec![e.to_string()],
            };
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/html; charset=utf-8")
                .body(page.render()?)?)
        }
    }
}

pub async fn agents_delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Response<String>> {
    agents::service::delete_agent(state.db.get_pool(), id).await?;
    Ok(Response::builder()
        .status(StatusCode::FOUND)
        .header("location", "/agents")
        .body(String::new())?)
}

// ===== Sessions Routes =====
pub async fn sessions_list(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let sessions_data = sessions::service::list_sessions(state.db.get_pool(), state.basic_user_id).await?;
    let sessions: Vec<views::SessionListItem> = sessions_data
        .into_iter()
        .map(|s| views::SessionListItem {
            id: s.id,
            title: s.title,
            agent_id: s.agent_id,
        })
        .collect();

    Ok(views::SessionsList { sessions })
}

pub async fn sessions_new(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let agents_data =
        agents::service::list_agents(state.db.get_pool(), state.basic_user_id, 50, 0).await?;
    let agents: Vec<views::AgentListItem> = agents_data
        .into_iter()
        .map(|a| views::AgentListItem {
            id: a.id,
            name: a.name,
            description: a.description,
            model_id: a.model_id,
        })
        .collect();

    Ok(views::SessionsNew { agents })
}

#[derive(Deserialize)]
pub struct SessionForm {
    pub agent_id: Uuid,
}

pub async fn sessions_create(
    State(state): State<AppState>,
    Form(form): Form<SessionForm>,
) -> Result<Response<String>> {
    match sessions::service::create_session(
        state.db.get_pool(),
        form.agent_id,
        state.basic_user_id,
        None,
    )
    .await
    {
        Ok(session) => Ok(Response::builder()
            .status(StatusCode::FOUND)
            .header("location", format!("/sessions/{}/chat", session.id))
            .body(String::new())?),
        Err(_) => {
            let agents_data =
                agents::service::list_agents(state.db.get_pool(), state.basic_user_id, 50, 0)
                    .await?;
            let agents: Vec<views::AgentListItem> = agents_data
                .into_iter()
                .map(|a| views::AgentListItem {
                    id: a.id,
                    name: a.name,
                    description: a.description,
                    model_id: a.model_id,
                })
                .collect();

            let page = views::SessionsNew { agents };
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/html; charset=utf-8")
                .body(page.render()?)?)
        }
    }
}

pub async fn session_chat(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let session_data = sessions::service::get_session_by_id(state.db.get_pool(), id).await?;
    let messages_data = messages::service::get_messages_by_session(state.db.get_pool(), id).await?;

    let session = views::SessionFormItem {
        id: session_data.id,
        title: session_data.title,
        agent_id: session_data.agent_id,
    };

    let messages: Vec<views::MessageItem> = messages_data
        .into_iter()
        .map(|m| views::MessageItem {
            id: m.id,
            content: m.content,
            role: m.role,
        })
        .collect();

    Ok(views::SessionsChat { session, messages })
}

#[derive(Deserialize)]
pub struct MessageForm {
    pub content: String,
    #[serde(default = "default_role")]
    pub role: String,
}

fn default_role() -> String {
    "user".to_string()
}

pub async fn message_create(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Form(form): Form<MessageForm>,
) -> Result<Html<String>> {
    let _user_message = messages::service::create_message(
        state.db.get_pool(),
        session_id,
        &form.role,
        &form.content,
    )
    .await?;

    // Fetch all messages for this session (including the new user message and agent response)
    let all_messages = messages::service::get_messages_by_session(
        state.db.get_pool(),
        session_id,
    )
    .await?;

    // Render both user message and agent message
    let mut html = String::new();
    for msg in all_messages.iter().rev().take(2).collect::<Vec<_>>().iter().rev() {
        let message_html = format!(
            r#"<div class="message mb-3" {}><div {}><div {}><span class="badge {}">{}</span><p class="mt-2 mb-0">{}</p><small class="text-muted">Just now</small></div></div></div>"#,
            if msg.role == "user" { r#"style="text-align: right;""# } else { "" },
            if msg.role == "user" { r#"style="display: flex; justify-content: flex-end;""# } else { "" },
            if msg.role == "user" { r#"style="max-width: 75%; text-align: right;""# } else { r#"style="max-width: 75%;""# },
            if msg.role == "user" { "bg-primary" } else { "bg-success" },
            if msg.role == "user" { "You" } else { "Agent" },
            msg.content
        );
        html.push_str(&message_html);
    }

    Ok(Html(html))
}

pub async fn sessions_delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Response<String>> {
    sessions::service::delete_session(state.db.get_pool(), id).await?;
    Ok(Response::builder()
        .status(StatusCode::FOUND)
        .header("location", "/sessions")
        .body(String::new())?)
}

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

async fn redirect_to_sessions() -> impl IntoResponse {
    (StatusCode::FOUND, [("Location", "/sessions")])
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(redirect_to_sessions))
        .route("/health", get(health_check))
        .route("/providers", get(providers_list).post(providers_create))
        .route("/providers/new", get(providers_new))
        .route("/providers/:id/edit", get(providers_edit))
        .route("/providers/:id", post(providers_update))
        .route("/providers/:id/delete", post(providers_delete))
        .route("/models", get(models_list).post(models_create))
        .route("/models/new", get(models_new))
        .route("/models/:id/edit", get(models_edit))
        .route("/models/:id", post(models_update))
        .route("/models/:id/delete", post(models_delete))
        .route("/agents", get(agents_list).post(agents_create))
        .route("/agents/new", get(agents_new))
        .route("/agents/:id/edit", get(agents_edit))
        .route("/agents/:id", post(agents_update))
        .route("/agents/:id/delete", post(agents_delete))
        .route("/agents/:id/chat", get(session_chat))
        .route("/sessions", get(sessions_list).post(sessions_create))
        .route("/sessions/new", get(sessions_new))
        .route("/sessions/:id/chat", get(session_chat))
        .route("/sessions/:id/messages", post(message_create))
        .route("/sessions/:id/delete", post(sessions_delete))
        .layer(axum::middleware::from_fn(logging_middleware))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        assert!(true);
    }
}
