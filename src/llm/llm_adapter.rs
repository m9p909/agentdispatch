use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone)]
pub struct LlmAdapter {
    client: reqwest::Client,
}

impl LlmAdapter {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn call_api(
        &self,
        api_key: &str,
        model_identifier: &str,
        system_prompt: &str,
        messages: Vec<LlmMessage>,
        api_endpoint: &str,
    ) -> Result<LlmResponse> {
        call_llm_api_with_client(&self.client, api_key, model_identifier, system_prompt, messages, api_endpoint).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
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
        content: m.content,
    }));

    let request = ChatCompletionRequest {
        model: model_identifier.to_string(),
        messages: all_messages,
    };

    let url = format!("{}/chat/completions", api_endpoint.trim_end_matches('/'));

    tracing::info!("Calling LLM API at: {}", url);
    tracing::debug!("Request model: {}, messages: {}", model_identifier, request.messages.len());

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

    tracing::info!("LLM API response received: {} tokens",
        usage.as_ref().and_then(|u| u.get("total_tokens")).unwrap_or(&json!(0)));

    Ok(LlmResponse { content, usage })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_message_struct() {
        let msg = LlmMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
        };

        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_llm_request_struct() {
        let req = LlmRequest {
            model: "gpt-4".to_string(),
            messages: vec![LlmMessage {
                role: "user".to_string(),
                content: "Hi".to_string(),
            }],
            system_prompt: "Be helpful".to_string(),
        };

        assert_eq!(req.model, "gpt-4");
        assert_eq!(req.messages.len(), 1);
    }

    #[tokio::test]
    async fn test_call_llm_api_empty_api_key() {
        let adapter = LlmAdapter::new();
        let result = adapter.call_api("", "gpt-4", "prompt", vec![], "https://api.openai.com/v1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_call_llm_api_empty_model() {
        let adapter = LlmAdapter::new();
        let result = adapter.call_api("test-key", "", "prompt", vec![], "https://api.openai.com/v1").await;
        assert!(result.is_err());
    }
}
