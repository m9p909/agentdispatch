use std::time::Duration;

use reqwest::Client;
use serde::Deserialize;

use crate::error::{AppError, Result};

// ===== DTOs =====

#[derive(Debug, Clone, Deserialize)]
pub struct BotInfo {
    pub id: i64,
    pub username: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramUpdate {
    pub update_id: i64,
    pub message: Option<TelegramMessage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramMessage {
    pub message_id: i64,
    pub from: Option<TelegramUser>,
    pub chat: TelegramChat,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramUser {
    pub id: i64,
    pub is_bot: bool,
    pub first_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramChat {
    pub id: i64,
}

#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    ok: bool,
    result: Option<T>,
    description: Option<String>,
}

// ===== Adapter =====

#[derive(Debug, Clone)]
pub struct TelegramAdapter {
    client: Client,
}

impl TelegramAdapter {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    fn api_url(token: &str, method: &str) -> String {
        format!("https://api.telegram.org/bot{}/{}", token, method)
    }

    /// GET /getMe — validates the bot token and returns bot info.
    pub async fn verify_token(&self, token: &str) -> Result<BotInfo> {
        let url = Self::api_url(token, "getMe");
        let resp: ApiResponse<BotInfo> = self
            .client
            .get(&url)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Telegram request failed: {}", e)))?
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Telegram response parse failed: {}", e)))?;

        if resp.ok {
            resp.result
                .ok_or_else(|| AppError::Internal("Empty result from getMe".to_string()))
        } else {
            Err(AppError::Validation(format!(
                "Invalid bot token: {}",
                resp.description.unwrap_or_default()
            )))
        }
    }

    /// GET /getUpdates?offset=N&timeout=30 — long-polls for new messages.
    pub async fn get_updates(&self, token: &str, offset: i64) -> Result<Vec<TelegramUpdate>> {
        let url = Self::api_url(token, "getUpdates");
        let resp: ApiResponse<Vec<TelegramUpdate>> = self
            .client
            .get(&url)
            .query(&[
                ("offset", offset.to_string()),
                ("timeout", "30".to_string()),
            ])
            .timeout(Duration::from_secs(35))
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Telegram request failed: {}", e)))?
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Telegram response parse failed: {}", e)))?;

        if resp.ok {
            Ok(resp.result.unwrap_or_default())
        } else {
            Err(AppError::Internal(format!(
                "getUpdates failed: {}",
                resp.description.unwrap_or_default()
            )))
        }
    }

    /// POST /sendChatAction — shows the "typing…" indicator in the chat.
    /// The indicator expires after ~5 s; call repeatedly to keep it alive.
    pub async fn send_typing(&self, token: &str, chat_id: i64) -> Result<()> {
        let url = Self::api_url(token, "sendChatAction");
        let resp: ApiResponse<bool> = self
            .client
            .post(&url)
            .timeout(Duration::from_secs(10))
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "action": "typing",
            }))
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Telegram request failed: {}", e)))?
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Telegram response parse failed: {}", e)))?;

        if resp.ok {
            Ok(())
        } else {
            Err(AppError::Internal(format!(
                "sendChatAction failed: {}",
                resp.description.unwrap_or_default()
            )))
        }
    }

    /// POST /sendMessage — sends a text reply to the user.
    pub async fn send_message(&self, token: &str, chat_id: i64, text: &str) -> Result<()> {
        let url = Self::api_url(token, "sendMessage");
        let resp: ApiResponse<serde_json::Value> = self
            .client
            .post(&url)
            .timeout(Duration::from_secs(10))
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "text": text,
            }))
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Telegram request failed: {}", e)))?
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Telegram response parse failed: {}", e)))?;

        if resp.ok {
            Ok(())
        } else {
            Err(AppError::Internal(format!(
                "sendMessage failed: {}",
                resp.description.unwrap_or_default()
            )))
        }
    }
}
