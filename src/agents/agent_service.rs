use super::db;
use crate::db::Database;
use crate::error::{AppError, Result};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct AgentResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub model_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: String,
}

#[derive(Clone)]
pub struct AgentService {
    pub db: Database,
}

impl AgentService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn create_agent(
        &self,
        user_id: Uuid,
        model_id: Uuid,
        name: &str,
        description: Option<&str>,
        system_prompt: &str,
    ) -> Result<AgentResponse> {
        if name.is_empty() {
            return Err(AppError::Validation("Name is required".to_string()));
        }
        if system_prompt.is_empty() {
            return Err(AppError::Validation(
                "System prompt is required".to_string(),
            ));
        }

        let req = db::CreateAgentRequest {
            user_id,
            model_id,
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            system_prompt: system_prompt.to_string(),
        };

        let agent = db::create_agent(self.db.get_pool(), &req)
            .await
            .map_err(AppError::Database)?;

        Ok(AgentResponse {
            id: agent.id,
            user_id: agent.user_id,
            model_id: agent.model_id,
            parent_id: agent.parent_id,
            name: agent.name,
            description: agent.description,
            system_prompt: agent.system_prompt,
        })
    }

    pub async fn list_agents(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AgentResponse>> {
        let agents = db::list_agents(self.db.get_pool(), user_id, limit, offset)
            .await
            .map_err(AppError::Database)?;

        Ok(agents
            .into_iter()
            .map(|a| AgentResponse {
                id: a.id,
                user_id: a.user_id,
                model_id: a.model_id,
                parent_id: a.parent_id,
                name: a.name,
                description: a.description,
                system_prompt: a.system_prompt,
            })
            .collect())
    }

    pub async fn get_agent_by_id(&self, id: Uuid) -> Result<AgentResponse> {
        let agent = db::get_agent_by_id(self.db.get_pool(), id)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("Agent not found".to_string()))?;

        Ok(AgentResponse {
            id: agent.id,
            user_id: agent.user_id,
            model_id: agent.model_id,
            parent_id: agent.parent_id,
            name: agent.name,
            description: agent.description,
            system_prompt: agent.system_prompt,
        })
    }

    pub async fn update_agent(
        &self,
        id: Uuid,
        model_id: Uuid,
        name: &str,
        description: Option<&str>,
        system_prompt: &str,
    ) -> Result<AgentResponse> {
        if name.is_empty() {
            return Err(AppError::Validation("Name is required".to_string()));
        }

        let agent = db::update_agent(self.db.get_pool(), id, model_id, name, description, system_prompt)
            .await
            .map_err(AppError::Database)?;

        Ok(AgentResponse {
            id: agent.id,
            user_id: agent.user_id,
            model_id: agent.model_id,
            parent_id: agent.parent_id,
            name: agent.name,
            description: agent.description,
            system_prompt: agent.system_prompt,
        })
    }

    pub async fn delete_agent(&self, id: Uuid) -> Result<()> {
        let rows = db::delete_agent(self.db.get_pool(), id)
            .await
            .map_err(AppError::Database)?;

        if rows == 0 {
            return Err(AppError::NotFound("Agent not found".to_string()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_response() {
        let resp = AgentResponse {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            model_id: Uuid::new_v4(),
            parent_id: None,
            name: "Support Bot".to_string(),
            description: Some("Customer support".to_string()),
            system_prompt: "Be helpful".to_string(),
        };

        assert_eq!(resp.name, "Support Bot");
    }
}
