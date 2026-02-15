use super::db;
use crate::error::{AppError, Result};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Serialize)]
pub struct SessionResponse {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub user_id: Uuid,
    pub title: Option<String>,
    pub is_active: bool,
}

pub async fn create_session(
    pool: &PgPool,
    agent_id: Uuid,
    user_id: Uuid,
    title: Option<&str>,
) -> Result<SessionResponse> {
    let req = db::CreateSessionRequest {
        agent_id,
        user_id,
        title: title.map(|s| s.to_string()),
    };

    let session = db::create_session(pool, &req)
        .await
        .map_err(AppError::Database)?;

    Ok(SessionResponse {
        id: session.id,
        agent_id: session.agent_id,
        user_id: session.user_id,
        title: session.title,
        is_active: session.is_active,
    })
}

pub async fn list_sessions(pool: &PgPool, user_id: Uuid) -> Result<Vec<SessionResponse>> {
    let sessions = db::list_sessions(pool, user_id)
        .await
        .map_err(AppError::Database)?;

    Ok(sessions
        .into_iter()
        .map(|s| SessionResponse {
            id: s.id,
            agent_id: s.agent_id,
            user_id: s.user_id,
            title: s.title,
            is_active: s.is_active,
        })
        .collect())
}

pub async fn get_session_by_id(pool: &PgPool, id: Uuid) -> Result<SessionResponse> {
    let session = db::get_session_by_id(pool, id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Session not found".to_string()))?;

    Ok(SessionResponse {
        id: session.id,
        agent_id: session.agent_id,
        user_id: session.user_id,
        title: session.title,
        is_active: session.is_active,
    })
}

pub async fn end_session(pool: &PgPool, id: Uuid) -> Result<()> {
    db::end_session(pool, id)
        .await
        .map_err(AppError::Database)?;

    Ok(())
}

pub async fn delete_session(pool: &PgPool, id: Uuid) -> Result<()> {
    let rows = db::delete_session(pool, id)
        .await
        .map_err(AppError::Database)?;

    if rows == 0 {
        return Err(AppError::NotFound("Session not found".to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_response() {
        let resp = SessionResponse {
            id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            title: Some("New Chat".to_string()),
            is_active: true,
        };

        assert!(resp.is_active);
    }
}
