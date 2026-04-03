use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Message {
    pub id: Uuid,
    pub session_id: Uuid,
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMessageRequest {
    pub session_id: Uuid,
    pub role: String,
    pub content: String,
}

pub async fn create_message(
    pool: &PgPool,
    req: &CreateMessageRequest,
) -> Result<Message, sqlx::Error> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query_as::<_, Message>(
        r#"
        INSERT INTO messages (id, session_id, role, content, timestamp, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(req.session_id)
    .bind(&req.role)
    .bind(&req.content)
    .bind(now)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
}

pub struct CreateMessageWithMetaRequest {
    pub session_id: Uuid,
    pub role: String,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
}

pub async fn create_message_with_meta(
    pool: &PgPool,
    req: &CreateMessageWithMetaRequest,
) -> Result<Message, sqlx::Error> {
    let id = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query_as::<_, Message>(
        r#"
        INSERT INTO messages (id, session_id, role, content, timestamp, metadata, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(req.session_id)
    .bind(&req.role)
    .bind(&req.content)
    .bind(now)
    .bind(&req.metadata)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
}

pub async fn get_message_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Message>, sqlx::Error> {
    sqlx::query_as::<_, Message>("SELECT * FROM messages WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn get_messages_by_session(
    pool: &PgPool,
    session_id: Uuid,
) -> Result<Vec<Message>, sqlx::Error> {
    sqlx::query_as::<_, Message>(
        "SELECT * FROM messages WHERE session_id = $1 ORDER BY timestamp ASC",
    )
    .bind(session_id)
    .fetch_all(pool)
    .await
}

pub async fn delete_message(pool: &PgPool, id: Uuid) -> Result<u64, sqlx::Error> {
    sqlx::query("DELETE FROM messages WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map(|r| r.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_struct() {
        let msg = Message {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            role: "user".to_string(),
            content: "Hello".to_string(),
            timestamp: Utc::now(),
            metadata: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");
    }
}
