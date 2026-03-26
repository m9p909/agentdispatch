use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub user_id: Uuid,
    pub title: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub agent_id: Uuid,
    pub user_id: Uuid,
    pub title: Option<String>,
}

pub async fn create_session(
    pool: &PgPool,
    req: &CreateSessionRequest,
) -> Result<Session, sqlx::Error> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query_as::<_, Session>(
        r#"
        INSERT INTO sessions (id, agent_id, user_id, title, started_at, is_active, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(req.agent_id)
    .bind(req.user_id)
    .bind(&req.title)
    .bind(now)
    .bind(true)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
}

pub async fn get_session_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Session>, sqlx::Error> {
    sqlx::query_as::<_, Session>("SELECT * FROM sessions WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn list_sessions(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<Session>, sqlx::Error> {
    sqlx::query_as::<_, Session>(
        "SELECT * FROM sessions WHERE user_id = $1 ORDER BY started_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn end_session(pool: &PgPool, id: Uuid) -> Result<Session, sqlx::Error> {
    let now = Utc::now();

    sqlx::query_as::<_, Session>(
        r#"
        UPDATE sessions
        SET ended_at = $1, is_active = $2, updated_at = $3
        WHERE id = $4
        RETURNING *
        "#,
    )
    .bind(now)
    .bind(false)
    .bind(now)
    .bind(id)
    .fetch_one(pool)
    .await
}

pub async fn delete_session(pool: &PgPool, id: Uuid) -> Result<u64, sqlx::Error> {
    sqlx::query("DELETE FROM sessions WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map(|r| r.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_struct() {
        let session = Session {
            id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            title: Some("Chat".to_string()),
            started_at: Utc::now(),
            ended_at: None,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(session.is_active);
        assert!(session.ended_at.is_none());
    }
}
