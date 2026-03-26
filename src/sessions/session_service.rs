use super::sessions_db as db;
use crate::db::Database;
use crate::error::{AppError, Result};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct SessionResponse {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub user_id: Uuid,
    pub title: Option<String>,
    pub is_active: bool,
}

#[derive(Clone)]
pub struct SessionService {
    pub db: Database,
}

impl SessionService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn create_session(
        &self,
        agent_id: Uuid,
        user_id: Uuid,
        title: Option<&str>,
    ) -> Result<SessionResponse> {
        let req = db::CreateSessionRequest {
            agent_id,
            user_id,
            title: title.map(|s| s.to_string()),
        };

        let session = db::create_session(self.db.get_pool(), &req)
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

    pub async fn list_sessions(&self, user_id: Uuid) -> Result<Vec<SessionResponse>> {
        let sessions = db::list_sessions(self.db.get_pool(), user_id)
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

    pub async fn get_session_by_id(&self, id: Uuid) -> Result<SessionResponse> {
        let session = db::get_session_by_id(self.db.get_pool(), id)
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

    pub async fn delete_session(&self, id: Uuid) -> Result<()> {
        let rows = db::delete_session(self.db.get_pool(), id)
            .await
            .map_err(AppError::Database)?;

        if rows == 0 {
            return Err(AppError::NotFound("Session not found".to_string()));
        }

        Ok(())
    }
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
