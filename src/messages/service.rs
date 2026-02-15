use super::db;
use crate::error::{AppError, Result};
use crate::{agents, llm, llm_models, llm_providers, sessions};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Serialize)]
pub struct MessageResponse {
    pub id: Uuid,
    pub session_id: Uuid,
    pub role: String,
    pub content: String,
}

pub async fn create_message(
    pool: &PgPool,
    session_id: Uuid,
    role: &str,
    content: &str,
) -> Result<MessageResponse> {
    if content.is_empty() {
        return Err(AppError::Validation("Content is required".to_string()));
    }
    if role.is_empty() {
        return Err(AppError::Validation("Role is required".to_string()));
    }

    let req = db::CreateMessageRequest {
        session_id,
        role: role.to_string(),
        content: content.to_string(),
    };

    let message = db::create_message(pool, &req)
        .await
        .map_err(AppError::Database)?;

    // If user message, generate agent response
    if message.role == "user" {
        generate_agent_response(pool, session_id).await?;
    }

    Ok(MessageResponse {
        id: message.id,
        session_id: message.session_id,
        role: message.role,
        content: message.content,
    })
}

async fn generate_agent_response(pool: &PgPool, session_id: Uuid) -> Result<()> {
    // Get session and agent
    let session = sessions::db::get_session_by_id(pool, session_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Session not found".to_string()))?;

    let agent = agents::db::get_agent_by_id(pool, session.agent_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Agent not found".to_string()))?;

    // Get model details
    let model = llm_models::db::get_model_by_id(pool, agent.model_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Model not found".to_string()))?;

    // Get provider details
    let provider = llm_providers::db::get_provider_by_id(pool, model.provider_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Provider not found".to_string()))?;

    // Get message history
    let message_records = db::get_messages_by_session(pool, session_id)
        .await
        .map_err(AppError::Database)?;

    let messages = message_records
        .into_iter()
        .map(|m| llm::service::LlmMessage {
            role: m.role,
            content: m.content,
        })
        .collect();

    // Call LLM API
    tracing::info!("Calling LLM for session {}: model={}", session_id, model.model_identifier);

    let response = llm::service::call_llm_api(
        &provider.api_key,
        &model.model_identifier,
        &agent.system_prompt,
        messages,
        provider.base_url.as_deref().unwrap_or("https://api.openai.com/v1"),
    )
    .await?;

    tracing::info!("LLM response received: {} characters", response.content.len());

    // Save agent response
    let agent_message_req = db::CreateMessageRequest {
        session_id,
        role: "assistant".to_string(),
        content: response.content,
    };

    db::create_message(pool, &agent_message_req)
        .await
        .map_err(AppError::Database)?;

    Ok(())
}

pub async fn get_messages_by_session(
    pool: &PgPool,
    session_id: Uuid,
) -> Result<Vec<MessageResponse>> {
    let messages = db::get_messages_by_session(pool, session_id)
        .await
        .map_err(AppError::Database)?;

    Ok(messages
        .into_iter()
        .map(|m| MessageResponse {
            id: m.id,
            session_id: m.session_id,
            role: m.role,
            content: m.content,
        })
        .collect())
}

pub async fn get_message_by_id(pool: &PgPool, id: Uuid) -> Result<MessageResponse> {
    let message = db::get_message_by_id(pool, id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Message not found".to_string()))?;

    Ok(MessageResponse {
        id: message.id,
        session_id: message.session_id,
        role: message.role,
        content: message.content,
    })
}

pub async fn delete_message(pool: &PgPool, id: Uuid) -> Result<()> {
    let rows = db::delete_message(pool, id)
        .await
        .map_err(AppError::Database)?;

    if rows == 0 {
        return Err(AppError::NotFound("Message not found".to_string()));
    }

    Ok(())
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
