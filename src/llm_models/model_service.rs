use super::models_db as db;
use crate::db::Database;
use crate::error::{AppError, Result};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct ModelResponse {
    pub id: Uuid,
    pub provider_id: Uuid,
    pub name: String,
    pub model_identifier: String,
}

#[derive(Clone)]
pub struct ModelService {
    pub db: Database,
}

impl ModelService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn create_model(
        &self,
        provider_id: Uuid,
        name: &str,
        model_identifier: &str,
    ) -> Result<ModelResponse> {
        if name.is_empty() {
            return Err(AppError::Validation("Name is required".to_string()));
        }
        if model_identifier.is_empty() {
            return Err(AppError::Validation(
                "Model identifier is required".to_string(),
            ));
        }

        let req = db::CreateModelRequest {
            provider_id,
            name: name.to_string(),
            model_identifier: model_identifier.to_string(),
        };

        let model = db::create_model(self.db.get_pool(), &req)
            .await
            .map_err(AppError::Database)?;

        Ok(ModelResponse {
            id: model.id,
            provider_id: model.provider_id,
            name: model.name,
            model_identifier: model.model_identifier,
        })
    }

    pub async fn list_models(&self) -> Result<Vec<ModelResponse>> {
        let models = db::list_models(self.db.get_pool())
            .await
            .map_err(AppError::Database)?;

        Ok(models
            .into_iter()
            .map(|m| ModelResponse {
                id: m.id,
                provider_id: m.provider_id,
                name: m.name,
                model_identifier: m.model_identifier,
            })
            .collect())
    }

    pub async fn get_model_by_id(&self, id: Uuid) -> Result<ModelResponse> {
        let model = db::get_model_by_id(self.db.get_pool(), id)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("Model not found".to_string()))?;

        Ok(ModelResponse {
            id: model.id,
            provider_id: model.provider_id,
            name: model.name,
            model_identifier: model.model_identifier,
        })
    }

    pub async fn update_model(
        &self,
        id: Uuid,
        provider_id: Uuid,
        name: &str,
        model_identifier: &str,
    ) -> Result<ModelResponse> {
        if name.is_empty() {
            return Err(AppError::Validation("Name is required".to_string()));
        }
        if model_identifier.is_empty() {
            return Err(AppError::Validation(
                "Model identifier is required".to_string(),
            ));
        }

        let model = db::update_model(self.db.get_pool(), id, provider_id, name, model_identifier)
            .await
            .map_err(AppError::Database)?;

        Ok(ModelResponse {
            id: model.id,
            provider_id: model.provider_id,
            name: model.name,
            model_identifier: model.model_identifier,
        })
    }

    pub async fn delete_model(&self, id: Uuid) -> Result<()> {
        let rows = db::delete_model(self.db.get_pool(), id)
            .await
            .map_err(AppError::Database)?;

        if rows == 0 {
            return Err(AppError::NotFound("Model not found".to_string()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_response() {
        let resp = ModelResponse {
            id: Uuid::new_v4(),
            provider_id: Uuid::new_v4(),
            name: "Claude 3".to_string(),
            model_identifier: "claude-3-opus".to_string(),
        };

        assert_eq!(resp.name, "Claude 3");
    }
}
