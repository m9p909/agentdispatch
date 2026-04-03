use super::agent_loop_service::{run_agent_loop, AgentConfig};
use super::messages_db as db;
use crate::crypto::CryptoService;
use crate::db::Database;
use crate::error::{AppError, Result};
use crate::llm::llm_adapter::LlmAdapter;
use crate::llm::tool_registry::ToolRegistry;
use crate::{agents, llm_models, llm_providers, sessions};
use async_stream::try_stream;
use futures_util::{Stream, StreamExt};
use serde::Serialize;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use uuid::Uuid;

#[derive(Serialize)]
pub struct MessageResponse {
    pub id: Uuid,
    pub session_id: Uuid,
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SseEvent {
    Token { delta: String },
    ToolCall { id: String, name: String, arguments: String },
    ToolResult { id: String, result: String },
    Done { message_id: Uuid },
    Error { message: String },
}

#[derive(Clone)]
pub struct MessageService {
    pub db: Database,
    pub llm: LlmAdapter,
    pub crypto: CryptoService,
    pub session_guards: Arc<Mutex<HashMap<Uuid, Arc<Semaphore>>>>,
}

impl MessageService {
    pub fn new(db: Database, llm: LlmAdapter, crypto: CryptoService) -> Self {
        Self {
            db,
            llm,
            crypto,
            session_guards: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn create_message(&self, session_id: Uuid, role: &str, content: &str) -> Result<MessageResponse> {
        if content.is_empty() {
            return Err(AppError::Validation("Content is required".to_string()));
        }
        if role.is_empty() {
            return Err(AppError::Validation("Role is required".to_string()));
        }
        let message = db::create_message(self.db.get_pool(), &db::CreateMessageRequest {
            session_id,
            role: role.to_string(),
            content: content.to_string(),
        })
        .await
        .map_err(AppError::Database)?;

        Ok(to_response(message))
    }

    pub fn stream_response(
        &self,
        session_id: Uuid,
        user_content: String,
        tools: ToolRegistry,
    ) -> impl Stream<Item = Result<SseEvent>> + Send + 'static {
        let svc = self.clone();
        try_stream! {
            let pool = svc.db.get_pool();

            let permit = acquire_session_permit(&svc.session_guards, session_id).await;
            if permit.is_none() {
                yield SseEvent::Error { message: "another stream is already active for this session".to_string() };
                return;
            }
            let _permit = permit;

            store_user_message(pool, session_id, user_content).await?;
            let config = load_agent_config(pool, session_id, &svc.crypto).await?;

            let mut agent_stream = std::pin::pin!(run_agent_loop(
                pool.clone(), svc.llm.clone(), config, session_id, tools,
            ));
            while let Some(event) = agent_stream.next().await {
                yield event?;
            }
        }
    }

    pub async fn stream_to_text(&self, session_id: Uuid, user_content: String) -> Result<String> {
        let tools = ToolRegistry::new();
        let mut stream = std::pin::pin!(self.stream_response(session_id, user_content, tools));
        let mut final_text = String::new();
        while let Some(event) = stream.next().await {
            match event? {
                SseEvent::Token { delta } => final_text.push_str(&delta),
                SseEvent::Done { .. } => break,
                SseEvent::Error { message } => return Err(AppError::Internal(message)),
                SseEvent::ToolCall { .. } | SseEvent::ToolResult { .. } => {}
            }
        }
        Ok(final_text)
    }

    pub async fn get_messages_by_session(&self, session_id: Uuid) -> Result<Vec<MessageResponse>> {
        let messages = db::get_messages_by_session(self.db.get_pool(), session_id)
            .await
            .map_err(AppError::Database)?;
        Ok(messages.into_iter().map(to_response).collect())
    }

    pub async fn get_message_by_id(&self, id: Uuid) -> Result<MessageResponse> {
        let message = db::get_message_by_id(self.db.get_pool(), id)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("Message not found".to_string()))?;
        Ok(to_response(message))
    }

    pub async fn delete_message(&self, id: Uuid) -> Result<()> {
        let rows = db::delete_message(self.db.get_pool(), id)
            .await
            .map_err(AppError::Database)?;
        if rows == 0 {
            return Err(AppError::NotFound("Message not found".to_string()));
        }
        Ok(())
    }
}

async fn acquire_session_permit(
    guards: &Arc<Mutex<HashMap<Uuid, Arc<Semaphore>>>>,
    session_id: Uuid,
) -> Option<tokio::sync::OwnedSemaphorePermit> {
    let mut map = guards.lock().await;
    let sem = map.entry(session_id)
        .or_insert_with(|| Arc::new(Semaphore::new(1)))
        .clone();
    sem.try_acquire_owned().ok()
}

async fn store_user_message(pool: &PgPool, session_id: Uuid, content: String) -> Result<()> {
    db::create_message(pool, &db::CreateMessageRequest {
        session_id,
        role: "user".to_string(),
        content,
    })
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

async fn load_agent_config(pool: &PgPool, session_id: Uuid, crypto: &CryptoService) -> Result<AgentConfig> {
    let session = sessions::sessions_db::get_session_by_id(pool, session_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Session not found".to_string()))?;

    let agent = agents::agents_db::get_agent_by_id(pool, session.agent_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Agent not found".to_string()))?;

    let model = llm_models::models_db::get_model_by_id(pool, agent.model_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Model not found".to_string()))?;

    let provider = llm_providers::providers_db::get_provider_by_id(pool, model.provider_id, crypto)
        .await?
        .ok_or_else(|| AppError::NotFound("Provider not found".to_string()))?;

    Ok(AgentConfig {
        api_key: provider.api_key,
        api_endpoint: provider.base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
        model_identifier: model.model_identifier,
        system_prompt: agent.system_prompt,
    })
}

fn to_response(m: db::Message) -> MessageResponse {
    MessageResponse { id: m.id, session_id: m.session_id, role: m.role, content: m.content }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_response() {
        let resp = MessageResponse {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            role: "assistant".to_string(),
            content: "Hello there!".to_string(),
        };
        assert_eq!(resp.role, "assistant");
    }
}
