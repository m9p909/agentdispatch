use super::messages_db as db;
use crate::error::{AppError, Result};
use crate::llm::tool_registry::{ToolCall, ToolRegistry};
use crate::messages::message_service::SseEvent;
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

struct PartialToolCall {
    id: String,
    name: String,
    arguments: String,
}

pub struct ToolCallAccumulator {
    calls: HashMap<usize, PartialToolCall>,
}

impl ToolCallAccumulator {
    pub fn new() -> Self {
        Self { calls: HashMap::new() }
    }

    pub fn feed(&mut self, index: usize, id: Option<String>, name: Option<String>, arguments: Option<String>) {
        let entry = self.calls.entry(index).or_insert(PartialToolCall {
            id: String::new(),
            name: String::new(),
            arguments: String::new(),
        });
        if let Some(i) = id { entry.id = i; }
        if let Some(n) = name { entry.name = n; }
        if let Some(a) = arguments { entry.arguments.push_str(&a); }
    }

    pub fn finish(self) -> Vec<ToolCall> {
        let mut calls: Vec<(usize, ToolCall)> = self
            .calls
            .into_iter()
            .map(|(idx, p)| (idx, ToolCall { id: p.id, name: p.name, arguments: p.arguments }))
            .collect();
        calls.sort_by_key(|(idx, _)| *idx);
        calls.into_iter().map(|(_, c)| c).collect()
    }
}

/// Persist tool call assistant message + execute each call + persist tool results.
/// Yields SSE events for each call and result.
pub async fn execute_tool_calls(
    pool: &PgPool,
    session_id: Uuid,
    calls: &[ToolCall],
    tools: &ToolRegistry,
) -> Result<Vec<SseEvent>> {
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

    let mut events = Vec::new();
    for call in calls {
        events.push(SseEvent::ToolCall {
            id: call.id.clone(),
            name: call.name.clone(),
            arguments: call.arguments.clone(),
        });
        let result = tools.execute(call).await;
        db::create_message_with_meta(pool, &db::CreateMessageWithMetaRequest {
            session_id,
            role: "tool".to_string(),
            content: result.output.clone(),
            metadata: Some(serde_json::json!({ "tool_call_id": call.id })),
        })
        .await
        .map_err(AppError::Database)?;
        events.push(SseEvent::ToolResult { id: result.id, result: result.output });
    }
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

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