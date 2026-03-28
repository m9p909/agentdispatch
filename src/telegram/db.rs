use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, Result};

// ===== Data Models =====

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TelegramConfig {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub bot_token: String, // encrypted at rest
    pub is_enabled: bool,
    pub last_update_id: i64,
    pub owner_instance_id: Option<String>,
    pub lease_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TelegramWhitelistEntry {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub telegram_user_id: i64,
    pub created_at: DateTime<Utc>,
}

// ===== Connector CRUD =====

pub async fn create_config(
    pool: &PgPool,
    agent_id: Uuid,
    encrypted_token: &str,
) -> Result<TelegramConfig> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query_as::<_, TelegramConfig>(
        r#"
        INSERT INTO telegram_configs (id, agent_id, bot_token, is_enabled, last_update_id, created_at, updated_at)
        VALUES ($1, $2, $3, true, 0, $4, $5)
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(agent_id)
    .bind(encrypted_token)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn get_config_by_id(pool: &PgPool, id: Uuid) -> Result<Option<TelegramConfig>> {
    sqlx::query_as::<_, TelegramConfig>("SELECT * FROM telegram_configs WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn get_config_by_agent_id(pool: &PgPool, agent_id: Uuid) -> Result<Option<TelegramConfig>> {
    sqlx::query_as::<_, TelegramConfig>("SELECT * FROM telegram_configs WHERE agent_id = $1")
        .bind(agent_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn list_configs(pool: &PgPool) -> Result<Vec<TelegramConfig>> {
    sqlx::query_as::<_, TelegramConfig>(
        "SELECT * FROM telegram_configs ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn set_enabled(
    pool: &PgPool,
    agent_id: Uuid,
    enabled: bool,
) -> Result<Option<TelegramConfig>> {
    sqlx::query_as::<_, TelegramConfig>(
        r#"
        UPDATE telegram_configs
        SET is_enabled = $1, updated_at = NOW()
        WHERE agent_id = $2
        RETURNING *
        "#,
    )
    .bind(enabled)
    .bind(agent_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn delete_config(pool: &PgPool, agent_id: Uuid) -> Result<u64> {
    sqlx::query("DELETE FROM telegram_configs WHERE agent_id = $1")
        .bind(agent_id)
        .execute(pool)
        .await
        .map(|r| r.rows_affected())
        .map_err(AppError::Database)
}

// ===== Whitelist =====

pub async fn add_whitelist_entry(
    pool: &PgPool,
    agent_id: Uuid,
    telegram_user_id: i64,
) -> Result<()> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO telegram_whitelists (id, agent_id, telegram_user_id, created_at)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (agent_id, telegram_user_id) DO NOTHING
        "#,
    )
    .bind(id)
    .bind(agent_id)
    .bind(telegram_user_id)
    .bind(now)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(())
}

pub async fn remove_whitelist_entry(
    pool: &PgPool,
    agent_id: Uuid,
    telegram_user_id: i64,
) -> Result<u64> {
    sqlx::query(
        "DELETE FROM telegram_whitelists WHERE agent_id = $1 AND telegram_user_id = $2",
    )
    .bind(agent_id)
    .bind(telegram_user_id)
    .execute(pool)
    .await
    .map(|r| r.rows_affected())
    .map_err(AppError::Database)
}

pub async fn get_whitelist(pool: &PgPool, agent_id: Uuid) -> Result<Vec<i64>> {
    let rows: Vec<(i64,)> = sqlx::query_as(
        "SELECT telegram_user_id FROM telegram_whitelists WHERE agent_id = $1 ORDER BY created_at ASC",
    )
    .bind(agent_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(rows.into_iter().map(|(id,)| id).collect())
}

// ===== Idempotency =====

/// Insert a processed-update marker. Returns true if newly inserted (first time),
/// false if already existed (already processed — skip).
pub async fn insert_processed_update(
    pool: &PgPool,
    connector_id: Uuid,
    telegram_message_id: i64,
) -> Result<bool> {
    let rows = sqlx::query(
        r#"
        INSERT INTO telegram_processed_updates (connector_id, telegram_message_id)
        VALUES ($1, $2)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(connector_id)
    .bind(telegram_message_id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?
    .rows_affected();

    Ok(rows > 0)
}

// ===== Session Lookup =====

/// Find an existing session by agent_id and title (used for per-telegram-user sessions).
pub async fn find_session_by_agent_and_title(
    pool: &PgPool,
    agent_id: Uuid,
    title: &str,
) -> Result<Option<crate::sessions::sessions_db::Session>> {
    sqlx::query_as::<_, crate::sessions::sessions_db::Session>(
        "SELECT * FROM sessions WHERE agent_id = $1 AND title = $2 LIMIT 1",
    )
    .bind(agent_id)
    .bind(title)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)
}
