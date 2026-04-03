use super::messages_db as db;
use super::tool_call_service::{execute_tool_calls, ToolCallAccumulator};
use crate::error::{AppError, Result};
use crate::llm::llm_adapter::{LlmAdapter, LlmMessage, LlmToolCall, StreamChunk};
use crate::llm::tool_registry::ToolRegistry;
use crate::messages::message_service::SseEvent;
use async_stream::try_stream;
use futures_util::{Stream, StreamExt};
use sqlx::PgPool;
use uuid::Uuid;

pub struct AgentConfig {
    pub api_key: String,
    pub api_endpoint: String,
    pub model_identifier: String,
    pub system_prompt: String,
}

/// Runs the agentic loop: stream LLM → handle tool calls → repeat until stop or max rounds.
pub fn run_agent_loop(
    pool: PgPool,
    llm: LlmAdapter,
    config: AgentConfig,
    session_id: Uuid,
    tools: ToolRegistry,
) -> impl Stream<Item = Result<SseEvent>> + Send + 'static {
    try_stream! {
        let tool_schemas = tools.get_schemas();

        for _round in 0..10 {
            let records = db::get_messages_by_session(&pool, session_id)
                .await
                .map_err(AppError::Database)?;
            let messages = records.iter().map(build_llm_message).collect::<Vec<_>>();

            let chunk_stream = llm.stream_api(
                config.api_key.clone(),
                config.model_identifier.clone(),
                config.system_prompt.clone(),
                messages,
                tool_schemas.clone(),
                config.api_endpoint.clone(),
            );
            tokio::pin!(chunk_stream);

            let mut content_buf = String::new();
            let mut acc = ToolCallAccumulator::new();
            let mut finish = String::new();
            let mut errored = false;

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
                        if !content_buf.is_empty() {
                            db::create_message_with_meta(&pool, &db::CreateMessageWithMetaRequest {
                                session_id,
                                role: "assistant".to_string(),
                                content: content_buf.clone(),
                                metadata: Some(serde_json::json!({ "error": err.to_string() })),
                            })
                            .await
                            .map_err(AppError::Database)?;
                        }
                        yield SseEvent::Error { message: err.to_string() };
                        yield SseEvent::Done { message_id: Uuid::nil() };
                        errored = true;
                        break;
                    }
                }
            }

            if errored {
                return;
            }

            if finish == "tool_calls" {
                let calls = acc.finish();
                let events = execute_tool_calls(&pool, session_id, &calls, &tools).await?;
                for event in events {
                    yield event;
                }
                continue;
            }

            let msg = db::create_message_with_meta(&pool, &db::CreateMessageWithMetaRequest {
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

        yield SseEvent::Error { message: "max tool iterations reached".to_string() };
        yield SseEvent::Done { message_id: Uuid::nil() };
    }
}

pub fn build_llm_message(m: &db::Message) -> LlmMessage {
    let tool_calls = m.metadata.as_ref().and_then(|meta| {
        meta.get("tool_calls")
            .and_then(|tc| serde_json::from_value::<Vec<LlmToolCall>>(tc.clone()).ok())
    });

    let tool_call_id = m.metadata.as_ref().and_then(|meta| {
        meta.get("tool_call_id").and_then(|id| id.as_str()).map(str::to_string)
    });

    let content = if tool_calls.is_some() { None } else { Some(m.content.clone()) };

    LlmMessage { role: m.role.clone(), content, tool_calls, tool_call_id }
}
