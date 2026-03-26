use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LlmModel {
    pub id: Uuid,
    pub provider_id: Uuid,
    pub name: String,
    pub model_identifier: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateModelRequest {
    pub provider_id: Uuid,
    pub name: String,
    pub model_identifier: String,
}

pub async fn create_model(
    pool: &PgPool,
    req: &CreateModelRequest,
) -> Result<LlmModel, sqlx::Error> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query_as::<_, LlmModel>(
        r#"
        INSERT INTO llm_models (id, provider_id, name, model_identifier, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(req.provider_id)
    .bind(&req.name)
    .bind(&req.model_identifier)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
}

pub async fn get_model_by_id(pool: &PgPool, id: Uuid) -> Result<Option<LlmModel>, sqlx::Error> {
    sqlx::query_as::<_, LlmModel>("SELECT * FROM llm_models WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn list_models(pool: &PgPool) -> Result<Vec<LlmModel>, sqlx::Error> {
    sqlx::query_as::<_, LlmModel>("SELECT * FROM llm_models ORDER BY created_at DESC")
        .fetch_all(pool)
        .await
}

pub async fn list_models_by_provider(
    pool: &PgPool,
    provider_id: Uuid,
) -> Result<Vec<LlmModel>, sqlx::Error> {
    sqlx::query_as::<_, LlmModel>("SELECT * FROM llm_models WHERE provider_id = $1 ORDER BY created_at DESC")
        .bind(provider_id)
        .fetch_all(pool)
        .await
}

pub async fn update_model(
    pool: &PgPool,
    id: Uuid,
    provider_id: Uuid,
    name: &str,
    model_identifier: &str,
) -> Result<LlmModel, sqlx::Error> {
    let now = Utc::now();

    sqlx::query_as::<_, LlmModel>(
        r#"
        UPDATE llm_models
        SET provider_id = $1, name = $2, model_identifier = $3, updated_at = $4
        WHERE id = $5
        RETURNING *
        "#,
    )
    .bind(provider_id)
    .bind(name)
    .bind(model_identifier)
    .bind(now)
    .bind(id)
    .fetch_one(pool)
    .await
}

pub async fn delete_model(pool: &PgPool, id: Uuid) -> Result<u64, sqlx::Error> {
    sqlx::query("DELETE FROM llm_models WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map(|r| r.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_model_struct() {
        let model = LlmModel {
            id: Uuid::new_v4(),
            provider_id: Uuid::new_v4(),
            name: "GPT-4".to_string(),
            model_identifier: "gpt-4".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(model.name, "GPT-4");
        assert_eq!(model.model_identifier, "gpt-4");
    }
}
