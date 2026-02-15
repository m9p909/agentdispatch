use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

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

pub async fn create_provider(
    pool: &PgPool,
    req: &CreateProviderRequest,
) -> Result<LlmProvider, sqlx::Error> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query_as::<_, LlmProvider>(
        r#"
        INSERT INTO llm_providers (id, name, type, api_key, base_url, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&req.name)
    .bind(&req.r#type)
    .bind(&req.api_key)
    .bind(&req.base_url)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
}

pub async fn get_provider_by_id(pool: &PgPool, id: Uuid) -> Result<Option<LlmProvider>, sqlx::Error> {
    sqlx::query_as::<_, LlmProvider>("SELECT * FROM llm_providers WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn list_providers(pool: &PgPool) -> Result<Vec<LlmProvider>, sqlx::Error> {
    sqlx::query_as::<_, LlmProvider>("SELECT * FROM llm_providers ORDER BY created_at DESC")
        .fetch_all(pool)
        .await
}

pub async fn update_provider(
    pool: &PgPool,
    id: Uuid,
    name: &str,
    r#type: &str,
    api_key: &str,
    base_url: Option<&str>,
) -> Result<LlmProvider, sqlx::Error> {
    let now = Utc::now();

    sqlx::query_as::<_, LlmProvider>(
        r#"
        UPDATE llm_providers
        SET name = $1, type = $2, api_key = $3, base_url = $4, updated_at = $5
        WHERE id = $6
        RETURNING *
        "#,
    )
    .bind(name)
    .bind(r#type)
    .bind(api_key)
    .bind(base_url)
    .bind(now)
    .bind(id)
    .fetch_one(pool)
    .await
}

pub async fn delete_provider(pool: &PgPool, id: Uuid) -> Result<u64, sqlx::Error> {
    sqlx::query("DELETE FROM llm_providers WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map(|r| r.rows_affected())
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
