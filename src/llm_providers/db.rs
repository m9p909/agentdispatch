use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
use crate::crypto::Cipher;
use crate::error::{AppError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LlmProvider {
    pub id: Uuid,
    pub name: String,
    pub r#type: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProviderRequest {
    pub name: String,
    pub r#type: String,
    pub api_key: String,
    pub base_url: Option<String>,
}

fn get_cipher() -> Result<Cipher> {
    Cipher::new().map_err(|e| AppError::Internal(format!("Cipher initialization failed: {}", e)))
}

fn is_encrypted(value: &str) -> bool {
    // Encrypted keys are hex-encoded and have minimum length (nonce + ciphertext)
    // Minimum: 12 bytes nonce + 16 bytes tag = 28 bytes = 56 hex chars
    value.len() >= 56 && value.chars().all(|c| c.is_ascii_hexdigit())
}

fn decrypt_if_needed(cipher: &Cipher, value: &str) -> Result<String> {
    if is_encrypted(value) {
        cipher.decrypt(value)
            .map_err(|e| AppError::Internal(format!("Decryption failed: {}", e)))
    } else {
        Ok(value.to_string())
    }
}

pub async fn create_provider(
    pool: &PgPool,
    req: &CreateProviderRequest,
) -> Result<LlmProvider> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    let cipher = get_cipher()?;
    let encrypted_key = cipher.encrypt(&req.api_key)
        .map_err(|e| AppError::Internal(format!("Encryption failed: {}", e)))?;

    let mut provider = sqlx::query_as::<_, LlmProvider>(
        r#"
        INSERT INTO llm_providers (id, name, type, api_key, base_url, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&req.name)
    .bind(&req.r#type)
    .bind(&encrypted_key)
    .bind(&req.base_url)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;

    provider.api_key = req.api_key.clone();
    Ok(provider)
}

pub async fn get_provider_by_id(pool: &PgPool, id: Uuid) -> Result<Option<LlmProvider>> {
    let mut provider = sqlx::query_as::<_, LlmProvider>("SELECT * FROM llm_providers WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;

    if let Some(ref mut p) = provider {
        let cipher = get_cipher()?;
        p.api_key = decrypt_if_needed(&cipher, &p.api_key)?;
    }

    Ok(provider)
}

pub async fn list_providers(pool: &PgPool) -> Result<Vec<LlmProvider>> {
    let mut providers = sqlx::query_as::<_, LlmProvider>("SELECT * FROM llm_providers ORDER BY created_at DESC")
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

    let cipher = get_cipher()?;

    for p in &mut providers {
        p.api_key = decrypt_if_needed(&cipher, &p.api_key)?;
    }

    Ok(providers)
}

pub async fn update_provider(
    pool: &PgPool,
    id: Uuid,
    name: &str,
    r#type: &str,
    api_key: &str,
    base_url: Option<&str>,
) -> Result<LlmProvider> {
    let now = Utc::now();

    let cipher = get_cipher()?;
    let encrypted_key = cipher.encrypt(api_key)
        .map_err(|e| AppError::Internal(format!("Encryption failed: {}", e)))?;

    let mut provider = sqlx::query_as::<_, LlmProvider>(
        r#"
        UPDATE llm_providers
        SET name = $1, type = $2, api_key = $3, base_url = $4, updated_at = $5
        WHERE id = $6
        RETURNING *
        "#,
    )
    .bind(name)
    .bind(r#type)
    .bind(&encrypted_key)
    .bind(base_url)
    .bind(now)
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;

    provider.api_key = api_key.to_string();
    Ok(provider)
}

pub async fn delete_provider(pool: &PgPool, id: Uuid) -> Result<u64> {
    sqlx::query("DELETE FROM llm_providers WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map(|r| r.rows_affected())
        .map_err(AppError::Database)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_provider_struct() {
        let provider = LlmProvider {
            id: Uuid::new_v4(),
            name: "OpenAI".to_string(),
            r#type: "openai".to_string(),
            api_key: "sk-test".to_string(),
            base_url: Some("https://api.openai.com".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(provider.name, "OpenAI");
        assert_eq!(provider.r#type, "openai");
    }

    #[test]
    fn test_create_provider_request() {
        let req = CreateProviderRequest {
            name: "Claude".to_string(),
            r#type: "anthropic".to_string(),
            api_key: "test-key".to_string(),
            base_url: Some("https://api.anthropic.com".to_string()),
        };

        assert_eq!(req.name, "Claude");
        assert_eq!(req.r#type, "anthropic");
    }
}
