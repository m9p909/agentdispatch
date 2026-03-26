use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Agent {
    pub id: Uuid,
    pub user_id: Uuid,
    pub model_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAgentRequest {
    pub user_id: Uuid,
    pub model_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: String,
}

pub async fn create_agent(
    pool: &PgPool,
    req: &CreateAgentRequest,
) -> Result<Agent, sqlx::Error> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query_as::<_, Agent>(
        r#"
        INSERT INTO agents (id, user_id, model_id, name, description, system_prompt, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(req.user_id)
    .bind(req.model_id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.system_prompt)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
}

pub async fn get_agent_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Agent>, sqlx::Error> {
    sqlx::query_as::<_, Agent>("SELECT * FROM agents WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn list_agents(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<Agent>, sqlx::Error> {
    sqlx::query_as::<_, Agent>(
        "SELECT * FROM agents WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

pub async fn count_agents(pool: &PgPool, user_id: Uuid) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agents WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await?;

    Ok(row.0)
}

pub async fn update_agent(
    pool: &PgPool,
    id: Uuid,
    model_id: Uuid,
    name: &str,
    description: Option<&str>,
    system_prompt: &str,
) -> Result<Agent, sqlx::Error> {
    let now = Utc::now();
    let parent_id = Uuid::new_v4();

    sqlx::query_as::<_, Agent>(
        r#"
        UPDATE agents
        SET model_id = $1, parent_id = $2, name = $3, description = $4, system_prompt = $5, updated_at = $6
        WHERE id = $7
        RETURNING *
        "#,
    )
    .bind(model_id)
    .bind(parent_id)
    .bind(name)
    .bind(description)
    .bind(system_prompt)
    .bind(now)
    .bind(id)
    .fetch_one(pool)
    .await
}

pub async fn delete_agent(pool: &PgPool, id: Uuid) -> Result<u64, sqlx::Error> {
    sqlx::query("DELETE FROM agents WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map(|r| r.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_struct() {
        let agent = Agent {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            model_id: Uuid::new_v4(),
            parent_id: None,
            name: "My Agent".to_string(),
            description: Some("Test agent".to_string()),
            system_prompt: "You are helpful".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(agent.name, "My Agent");
        assert!(agent.description.is_some());
    }
}
