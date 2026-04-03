use super::messages_db as db;
use crate::crypto::CryptoService;
use crate::db::Database;
use crate::error::{AppError, Result};
use crate::llm::llm_adapter::{LlmAdapter, LlmMessage, LlmToolCall, StreamChunk};
use crate::llm::tool_registry::{ToolCall, ToolRegistry};
use crate::{agents, llm_models, llm_providers, sessions};
use async_stream::try_stream;
use futures_util::Stream;
use futures_util::StreamExt;
use serde::Serialize;
use std::collections::HashMap;
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

struct PartialToolCall {
    id: String,
    name: String,
    arguments: String,
}

struct ToolCallAccumulator {
    calls: HashMap<usize, PartialToolCall>,
}

impl ToolCallAccumulator {
    fn new() -> Self {
        Self { calls: HashMap::new() }
    }

    fn feed(&mut self, index: usize, id: Option<String>, name: Option<String>, arguments: Option<String>) {
        let entry = self.calls.entry(index).or_insert(PartialToolCall {
            id: String::new(),
            name: String::new(),
            arguments: String::new(),
        });
        if let Some(i) = id { entry.id = i; }
        if let Some(n) = name { entry.name = n; }
        if let Some(a) = arguments { entry.arguments.push_str(&a); }
    }

    fn finish(self) -> Vec<ToolCall> {
        let mut calls: Vec<(usize, ToolCall)> = self
            .calls
            .into_iter()
            .map(|(idx, p)| {
                (idx, ToolCall { id: p.id, name: p.name, arguments: p.arguments })
            })
            .collect();
        calls.sort_by_key(|(idx, _)| *idx);
        calls.into_iter().map(|(_, c)| c).collect()
    }
}

#[derive(Clone)]
pub struct MessageService {
    pub db: Database,
    pub llm: LlmAdapter,
    pub crypto: CryptoService,
    // concurrency guard: one active stream per session
    pub session_guards: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<Uuid, std::sync::Arc<tokio::sync::Semaphore>>>>,
}

impl MessageService {
    pub fn new(db: Database, llm: LlmAdapter, crypto: CryptoService) -> Self {
        Self {
            db,
            llm,
            crypto,
            session_guards: std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub async fn create_message(
        &self,
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

        let message = db::create_message(self.db.get_pool(), &req)
            .await
            .map_err(AppError::Database)?;

        if message.role == "user" {
            self.generate_agent_response(session_id).await?;
        }

        Ok(MessageResponse {
            id: message.id,
            session_id: message.session_id,
            role: message.role,
            content: message.content,
        })
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

            // 0. Concurrency guard per session (single active stream)
            let permit = {
                let mut guards = svc.session_guards.lock().await;
                let sem = guards.entry(session_id)
                    .or_insert_with(|| std::sync::Arc::new(tokio::sync::Semaphore::new(1)))
                    .clone();
                match sem.clone().try_acquire_owned() {
                    Ok(p) => Some(p),
                    Err(_) => None,
                }
            };
            if permit.is_none() {
                yield SseEvent::Error { message: "another stream is already active for this session".to_string() };
                return;
            }
            let _permit = permit; // held until stream ends

            // 1. Store user message (only after guard acquired)
            db::create_message(pool, &db::CreateMessageRequest {
                session_id,
                role: "user".to_string(),
                content: user_content,
            })
            .await
            .map_err(AppError::Database)?;

            // 2. Load session → agent → model → provider
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

            let provider = llm_providers::providers_db::get_provider_by_id(pool, model.provider_id, &svc.crypto)
                .await?
                .ok_or_else(|| AppError::NotFound("Provider not found".to_string()))?;

            let api_key = provider.api_key.clone();
            let api_endpoint = provider.base_url.clone().unwrap_or_else(|| "https://api.openai.com/v1".to_string());
            let tool_schemas = tools.get_schemas();

            for _round in 0..10 {
                // 3. Load history
                let records = db::get_messages_by_session(pool, session_id)
                    .await
                    .map_err(AppError::Database)?;

                let messages = records.into_iter().map(|m| build_llm_message(&m)).collect::<Vec<_>>();

                // 4. Stream from LLM
                let chunk_stream = svc.llm.stream_api(
                    api_key.clone(),
                    model.model_identifier.clone(),
                    agent.system_prompt.clone(),
                    messages,
                    tool_schemas.clone(),
                    api_endpoint.clone(),
                );

                let mut content_buf = String::new();
                let mut acc = ToolCallAccumulator::new();
                let mut finish = String::new();

                tokio::pin!(chunk_stream);
                while let Some(chunk_res) = chunk_stream.next().await {
                    match chunk_res {
                        Ok(StreamChunk::Token(delta)) => {
                            content_buf.push_str(&delta);
                            yield SseEvent::Token { delta };
                        }
                        Ok(StreamChunk::ToolCallDelta { index, id, name, arguments }) => {
                            acc.feed(index, id, name, arguments);
                        }
                        Ok(StreamChunk::FinishReason(reason)) => {
                            finish = reason;
                        }
                        Ok(StreamChunk::Done) => break,
                        Err(err) => {
                            // Persist partial content if any, annotate with error

                            let mut saved_id: Option<Uuid> = None;
                            if !content_buf.is_empty() {
                                let msg = db::create_message_with_meta(pool, &db::CreateMessageWithMetaRequest {
                                    session_id,
                                    role: "assistant".to_string(),
                                    content: content_buf.clone(),
                                    metadata: Some(serde_json::json!({ "error": err.to_string() })),
                                })
                                .await
                                .map_err(AppError::Database)?;
                                saved_id = Some(msg.id);
                            }
                            // Notify UI of error, then Done to close
                            yield SseEvent::Error { message: err.to_string() };
                            yield SseEvent::Done { message_id: saved_id.unwrap_or_else(Uuid::nil) };
                            return;
                        }
                    }
                }

                if finish == "tool_calls" {
                    let calls = acc.finish();
                    let tool_calls_json = calls.iter().map(|c| serde_json::json!({
                        "id": c.id,
                        "type": "function",
                        "function": { "name": c.name, "arguments": c.arguments }
                    })).collect::<Vec<_>>();

                    db::create_message_with_meta(pool, &db::CreateMessageWithMetaRequest {
                        session_id,
                        role: "assistant".to_string(),
                        content: String::new(),
                        metadata: Some(serde_json::json!({ "tool_calls": tool_calls_json })),
                    })
                    .await
                    .map_err(AppError::Database)?;

                    for call in &calls {
                        yield SseEvent::ToolCall {
                            id: call.id.clone(),
                            name: call.name.clone(),
                            arguments: call.arguments.clone(),
                        };
                        let result = tools.execute(call).await;
                        db::create_message_with_meta(pool, &db::CreateMessageWithMetaRequest {
                            session_id,
                            role: "tool".to_string(),
                            content: result.output.clone(),
                            metadata: Some(serde_json::json!({ "tool_call_id": call.id })),
                        })
                        .await
                        .map_err(AppError::Database)?;
                        yield SseEvent::ToolResult { id: result.id, result: result.output };
                    }
                    // continue loop for next round
                } else {
                    // finish_reason == "stop" or stream ended
                    let msg = db::create_message_with_meta(pool, &db::CreateMessageWithMetaRequest {
                        session_id,
                        role: "assistant".to_string(),
                        content: content_buf,
                        metadata: None,
                    })
                    .await
                    .map_err(AppError::Database)?;

                    yield SseEvent::Done { message_id: msg.id };
                    return;
                }
            }

            // Emit an error then a terminal done so the UI can clear streaming state
            yield SseEvent::Error { message: "max tool iterations reached".to_string() };
            yield SseEvent::Done { message_id: Uuid::nil() };
            return;
        }
    }

    async fn generate_agent_response(&self, session_id: Uuid) -> Result<()> {
        let pool = self.db.get_pool();

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

        let provider = llm_providers::providers_db::get_provider_by_id(pool, model.provider_id, &self.crypto)
            .await?
            .ok_or_else(|| AppError::NotFound("Provider not found".to_string()))?;

        let message_records = db::get_messages_by_session(pool, session_id)
            .await
            .map_err(AppError::Database)?;

        let messages = message_records
            .into_iter()
            .map(|m| LlmMessage {
                role: m.role,
                content: Some(m.content),
                tool_calls: None,
                tool_call_id: None,
            })
            .collect();

        tracing::info!(
            "Calling LLM for session {}: model={}",
            session_id,
            model.model_identifier
        );

        let response = self
            .llm
            .call_api(
                &provider.api_key,
                &model.model_identifier,
                &agent.system_prompt,
                messages,
                provider
                    .base_url
                    .as_deref()
                    .unwrap_or("https://api.openai.com/v1"),
            )
            .await?;

        tracing::info!(
            "LLM response received: {} characters",
            response.content.len()
        );

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

    pub async fn get_messages_by_session(&self, session_id: Uuid) -> Result<Vec<MessageResponse>> {
        let messages = db::get_messages_by_session(self.db.get_pool(), session_id)
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

    pub async fn get_message_by_id(&self, id: Uuid) -> Result<MessageResponse> {
        let message = db::get_message_by_id(self.db.get_pool(), id)
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

fn build_llm_message(m: &db::Message) -> LlmMessage {
    let tool_calls = m.metadata.as_ref().and_then(|meta| {
        meta.get("tool_calls").and_then(|tc| {
            serde_json::from_value::<Vec<LlmToolCall>>(tc.clone()).ok()
        })
    });

    let tool_call_id = m.metadata.as_ref().and_then(|meta| {
        meta.get("tool_call_id").and_then(|id| id.as_str()).map(str::to_string)
    });

    let content = if tool_calls.is_some() {
        None
    } else {
        Some(m.content.clone())
    };

    LlmMessage { role: m.role.clone(), content, tool_calls, tool_call_id }
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

    #[test]
    fn test_tool_call_accumulator() {
        let mut acc = ToolCallAccumulator::new();
        acc.feed(0, Some("call_1".to_string()), Some("echo".to_string()), None);
        acc.feed(0, None, None, Some("{\"text\":\"he".to_string()));
        acc.feed(0, None, None, Some("llo\"}".to_string()));
        let calls = acc.finish();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_1");
        assert_eq!(calls[0].name, "echo");
        assert_eq!(calls[0].arguments, "{\"text\":\"hello\"}");
    }
}
