use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use super::db as telegram_db;
use super::telegram_adapter::TelegramAdapter;
use crate::crypto::CryptoService;
use crate::db::Database;
use crate::error::{AppError, Result};

// ===== Response type (never exposes the raw token) =====

#[derive(Debug, Clone, Serialize)]
pub struct ConnectorResponse {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub is_enabled: bool,
    pub masked_token: String, // e.g. "...ab1c"
    pub created_at: DateTime<Utc>,
}

fn mask_token(token: &str) -> String {
    if token.len() <= 4 {
        "...".to_string()
    } else {
        format!("...{}", &token[token.len() - 4..])
    }
}

// ===== Service =====

#[derive(Clone)]
pub struct TelegramConnectorService {
    pub db: Database,
    pub crypto: CryptoService,
    telegram: TelegramAdapter,
}

impl TelegramConnectorService {
    pub fn new(db: Database, crypto: CryptoService, telegram: TelegramAdapter) -> Self {
        Self { db, crypto, telegram }
    }

    // ===== Connector CRUD =====

    pub async fn create_connector(&self, agent_id: Uuid, bot_token: &str) -> Result<ConnectorResponse> {
        if bot_token.is_empty() {
            return Err(AppError::Validation("Bot token is required".to_string()));
        }

        // Validate with Telegram before storing
        self.telegram.verify_token(bot_token).await?;

        let encrypted = self
            .crypto
            .encrypt(bot_token)
            .map_err(|e| AppError::Internal(format!("Encryption failed: {}", e)))?;

        let config = telegram_db::create_config(self.db.get_pool(), agent_id, &encrypted).await?;

        Ok(ConnectorResponse {
            id: config.id,
            agent_id: config.agent_id,
            is_enabled: config.is_enabled,
            masked_token: mask_token(bot_token),
            created_at: config.created_at,
        })
    }

    pub async fn get_connector(&self, agent_id: Uuid) -> Result<Option<ConnectorResponse>> {
        let config = telegram_db::get_config_by_agent_id(self.db.get_pool(), agent_id).await?;

        match config {
            None => Ok(None),
            Some(c) => {
                let plain = self.crypto.decrypt(&c.bot_token).unwrap_or_default();
                Ok(Some(ConnectorResponse {
                    id: c.id,
                    agent_id: c.agent_id,
                    is_enabled: c.is_enabled,
                    masked_token: mask_token(&plain),
                    created_at: c.created_at,
                }))
            }
        }
    }

    pub async fn list_connectors(&self) -> Result<Vec<ConnectorResponse>> {
        let configs = telegram_db::list_configs(self.db.get_pool()).await?;

        let mut result = Vec::with_capacity(configs.len());
        for c in configs {
            let plain = self.crypto.decrypt(&c.bot_token).unwrap_or_default();
            result.push(ConnectorResponse {
                id: c.id,
                agent_id: c.agent_id,
                is_enabled: c.is_enabled,
                masked_token: mask_token(&plain),
                created_at: c.created_at,
            });
        }
        Ok(result)
    }

    pub async fn set_enabled(&self, agent_id: Uuid, enabled: bool) -> Result<ConnectorResponse> {
        let config = telegram_db::set_enabled(self.db.get_pool(), agent_id, enabled)
            .await?
            .ok_or_else(|| AppError::NotFound("Connector not found".to_string()))?;

        let plain = self.crypto.decrypt(&config.bot_token).unwrap_or_default();
        Ok(ConnectorResponse {
            id: config.id,
            agent_id: config.agent_id,
            is_enabled: config.is_enabled,
            masked_token: mask_token(&plain),
            created_at: config.created_at,
        })
    }

    pub async fn delete_connector(&self, agent_id: Uuid) -> Result<()> {
        let rows = telegram_db::delete_config(self.db.get_pool(), agent_id).await?;
        if rows == 0 {
            return Err(AppError::NotFound("Connector not found".to_string()));
        }
        Ok(())
    }

    /// Returns the plaintext bot token for a connector (used by the supervisor before polling).
    pub async fn get_decrypted_token(&self, connector_id: Uuid) -> Result<String> {
        let config = telegram_db::get_config_by_id(self.db.get_pool(), connector_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Connector not found".to_string()))?;

        self.crypto
            .decrypt(&config.bot_token)
            .map_err(|e| AppError::Internal(format!("Decryption failed: {}", e)))
    }

    // ===== Whitelist =====

    pub async fn add_whitelist_entry(&self, agent_id: Uuid, telegram_user_id: i64) -> Result<()> {
        telegram_db::add_whitelist_entry(self.db.get_pool(), agent_id, telegram_user_id).await
    }

    pub async fn remove_whitelist_entry(
        &self,
        agent_id: Uuid,
        telegram_user_id: i64,
    ) -> Result<()> {
        let rows =
            telegram_db::remove_whitelist_entry(self.db.get_pool(), agent_id, telegram_user_id)
                .await?;
        if rows == 0 {
            return Err(AppError::NotFound("Whitelist entry not found".to_string()));
        }
        Ok(())
    }

    pub async fn get_whitelist(&self, agent_id: Uuid) -> Result<Vec<i64>> {
        telegram_db::get_whitelist(self.db.get_pool(), agent_id).await
    }
}
