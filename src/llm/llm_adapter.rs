use crate::error::{AppError, Result};
use async_stream::try_stream;
use futures_util::Stream;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

#[derive(Clone)]
pub struct LlmAdapter {
    client: reqwest::Client,
}

impl LlmAdapter {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(30))
            .timeout(Duration::from_secs(300))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { client }
    }

    pub async fn call_api(
        &self,
        api_key: &str,
        model_identifier: &str,
        system_prompt: &str,
        messages: Vec<LlmMessage>,
        api_endpoint: &str,
    ) -> Result<LlmResponse> {
        call_llm_api_with_client(
            &self.client,
            api_key,
            model_identifier,
            system_prompt,
            messages,
            api_endpoint,
        )
        .await
    }

    pub fn stream_api(
        &self,
        api_key: String,
        model_identifier: String,
        system_prompt: String,
        messages: Vec<LlmMessage>,
        tool_schemas: Vec<serde_json::Value>,
        api_endpoint: String,
    ) -> impl Stream<Item = Result<StreamChunk>> + Send + 'static {
        let client = self.client.clone();
        try_stream! {
            let url = format!("{}/chat/completions", api_endpoint.trim_end_matches('/'));

            let mut all_messages = vec![serde_json::json!({
                "role": "system",
                "content": system_prompt,
            })];
            for m in &messages {
                all_messages.push(build_message_json(m));
            }

            let mut body = serde_json::json!({
                "model": model_identifier,
                "messages": all_messages,
                "stream": true,
            });
            if !tool_schemas.is_empty() {
                body["tools"] = serde_json::Value::Array(tool_schemas);
            }

            let response = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&body)
                .send()
                .await
                .map_err(|e| AppError::Internal(format!("LLM stream request failed: {}", e)))?;

            let byte_stream_result: std::result::Result<_, AppError> = if response.status().is_success() {
                Ok(response.bytes_stream())
            } else {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                Err(AppError::Internal(format!("LLM API {}: {}", status, text)))
            };
            let mut byte_stream = byte_stream_result?;
            let mut buf = String::new();

            while let Some(chunk) = byte_stream.next().await {
                let chunk = chunk.map_err(|e| AppError::Internal(format!("Stream read error: {}", e)))?;
                buf.push_str(&String::from_utf8_lossy(&chunk));

                loop {
                    if let Some(pos) = buf.find('\n') {
                        let line = buf[..pos].trim().to_string();
                        buf = buf[pos + 1..].to_string();

                        if line.is_empty() { continue; }
                        if !line.starts_with("data:") { continue; }

                        let data = line["data:".len()..].trim();
                        if data == "[DONE]" {
                            yield StreamChunk::Done;
                            return;
                        }

                        let v: serde_json::Value = serde_json::from_str::<serde_json::Value>(data)
                            .map_err(|e| AppError::Internal(format!("Invalid SSE JSON from LLM: {} | line={}", e, data)))?;
                        let chunks = parse_stream_chunks(&v);
                        for ch in chunks {
                            yield ch;
                        }
                    } else {
                        break;
                    }
                }
            }
        }
    }
}

fn build_message_json(m: &LlmMessage) -> serde_json::Value {
    let mut obj = serde_json::json!({ "role": m.role });
    if let Some(ref c) = m.content {
        obj["content"] = serde_json::Value::String(c.clone());
    } else {
        obj["content"] = serde_json::Value::Null;
    }
    if let Some(ref tcs) = m.tool_calls {
        obj["tool_calls"] = serde_json::to_value(tcs).unwrap_or_default();
    }
    if let Some(ref tcid) = m.tool_call_id {
        obj["tool_call_id"] = serde_json::Value::String(tcid.clone());
    }
    obj
}

fn parse_stream_chunks(v: &serde_json::Value) -> Vec<StreamChunk> {
    let choices = v.get("choices")?.as_array()?;
    let choice = choices.first()?;

    if let Some(reason) = choice.get("finish_reason").and_then(|r| r.as_str()) {
        if !reason.is_empty() {
            return Some(StreamChunk::FinishReason(reason.to_string()));
        }
    }

    let delta = choice.get("delta")?;

    if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
        if !content.is_empty() {
            return Some(StreamChunk::Token(content.to_string()));
        }
    }

    if let Some(tool_calls) = delta.get("tool_calls").and_then(|t| t.as_array()) {
        if let Some(tc) = tool_calls.first() {
            let index = tc.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
            let id = tc.get("id").and_then(|i| i.as_str()).map(str::to_string);
            let name = tc
                .get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                .map(str::to_string);
            let arguments = tc
                .get("function")
                .and_then(|f| f.get("arguments"))
                .and_then(|a| a.as_str())
                .map(str::to_string);
            return Some(StreamChunk::ToolCallDelta { index, id, name, arguments });
        }
    }

    None
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmToolCall {
    pub id: String,
    pub r#type: String,
    pub function: LlmFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: String,
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<LlmToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub model: String,
    pub messages: Vec<LlmMessage>,
    pub system_prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub usage: Option<serde_json::Value>,
}

pub enum StreamChunk {
    Token(String),
    ToolCallDelta {
        index: usize,
        id: Option<String>,
        name: Option<String>,
        arguments: Option<String>,
    },
    FinishReason(String),
    Done,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChatMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
}

async fn call_llm_api_with_client(
    client: &reqwest::Client,
    api_key: &str,
    model_identifier: &str,
    system_prompt: &str,
    messages: Vec<LlmMessage>,
    api_endpoint: &str,
) -> Result<LlmResponse> {
    if api_key.is_empty() {
        return Err(AppError::Validation("API key is required".to_string()));
    }

    if model_identifier.is_empty() {
        return Err(AppError::Validation(
            "Model identifier is required".to_string(),
        ));
    }

    let mut all_messages = vec![ChatMessage {
        role: "system".to_string(),
        content: system_prompt.to_string(),
    }];

    all_messages.extend(messages.into_iter().map(|m| ChatMessage {
        role: m.role,
        content: m.content.unwrap_or_default(),
    }));

    let request = ChatCompletionRequest {
        model: model_identifier.to_string(),
        messages: all_messages,
    };

    let url = format!("{}/chat/completions", api_endpoint.trim_end_matches('/'));

    tracing::info!("Calling LLM API at: {}", url);
    tracing::debug!(
        "Request model: {}, messages: {}",
        model_identifier,
        request.messages.len()
    );

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("LLM API request failed: {}", e);
            AppError::Internal(format!("LLM API call failed: {}", e))
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        tracing::error!("LLM API error: {} - {}", status, error_text);
        return Err(AppError::Internal(format!(
            "LLM API returned {}: {}",
            status, error_text
        )));
    }

    let chat_response: ChatCompletionResponse = response.json().await.map_err(|e| {
        tracing::error!("Failed to parse LLM response: {}", e);
        AppError::Internal(format!("Failed to parse LLM response: {}", e))
    })?;

    if chat_response.choices.is_empty() {
        return Err(AppError::Internal(
            "No choices in LLM response".to_string(),
        ));
    }

    let content = chat_response.choices[0].message.content.clone();
    let usage = chat_response.usage.map(|u| {
        json!({
            "prompt_tokens": u.prompt_tokens,
            "completion_tokens": u.completion_tokens,
            "total_tokens": u.total_tokens,
        })
    });

    tracing::info!(
        "LLM API response received: {} tokens",
        usage
            .as_ref()
            .and_then(|u| u.get("total_tokens"))
            .unwrap_or(&json!(0))
    );

    Ok(LlmResponse { content, usage })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_message_struct() {
        let msg = LlmMessage {
            role: "user".to_string(),
            content: Some("Hello".to_string()),
            tool_calls: None,
            tool_call_id: None,
        };

        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, Some("Hello".to_string()));
    }

    #[test]
    fn test_llm_request_struct() {
        let req = LlmRequest {
            model: "gpt-4".to_string(),
            messages: vec![LlmMessage {
                role: "user".to_string(),
                content: Some("Hi".to_string()),
                tool_calls: None,
                tool_call_id: None,
            }],
            system_prompt: "Be helpful".to_string(),
        };

        assert_eq!(req.model, "gpt-4");
        assert_eq!(req.messages.len(), 1);
    }

    #[tokio::test]
    async fn test_call_llm_api_empty_api_key() {
        let adapter = LlmAdapter::new();
        let result = adapter
            .call_api("", "gpt-4", "prompt", vec![], "https://api.openai.com/v1")
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_call_llm_api_empty_model() {
        let adapter = LlmAdapter::new();
        let result = adapter
            .call_api(
                "test-key",
                "",
                "prompt",
                vec![],
                "https://api.openai.com/v1",
            )
            .await;
        assert!(result.is_err());
    }
}
