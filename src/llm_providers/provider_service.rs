use super::providers_db as db;
use crate::crypto::CryptoService;
use crate::db::Database;
use crate::error::{AppError, Result};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct ProviderResponse {
    pub id: Uuid,
    pub name: String,
    pub r#type: String,
    pub base_url: Option<String>,
}

#[derive(Clone)]
pub struct ProviderService {
    pub db: Database,
    pub crypto: CryptoService,
}

impl ProviderService {
    pub fn new(db: Database, crypto: CryptoService) -> Self {
        Self { db, crypto }
    }

    pub async fn create_provider(
        &self,
        name: &str,
        r#type: &str,
        api_key: &str,
        base_url: Option<&str>,
    ) -> Result<ProviderResponse> {
        if name.is_empty() {
            return Err(AppError::Validation("Name is required".to_string()));
        }
        if r#type.is_empty() {
            return Err(AppError::Validation("Type is required".to_string()));
        }
        if api_key.is_empty() {
            return Err(AppError::Validation("API key is required".to_string()));
        }

        let req = db::CreateProviderRequest {
            name: name.to_string(),
            r#type: r#type.to_string(),
            api_key: api_key.to_string(),
            base_url: base_url.map(|s| s.to_string()),
        };

        let provider = db::create_provider(self.db.get_pool(), &req, &self.crypto).await?;

        Ok(ProviderResponse {
            id: provider.id,
            name: provider.name,
            r#type: provider.r#type,
            base_url: provider.base_url,
        })
    }

    pub async fn list_providers(&self) -> Result<Vec<ProviderResponse>> {
        let providers = db::list_providers(self.db.get_pool(), &self.crypto).await?;

        Ok(providers
            .into_iter()
            .map(|p| ProviderResponse {
                id: p.id,
                name: p.name,
                r#type: p.r#type,
                base_url: p.base_url,
            })
            .collect())
    }

    pub async fn get_provider_by_id(&self, id: Uuid) -> Result<ProviderResponse> {
        let provider = db::get_provider_by_id(self.db.get_pool(), id, &self.crypto)
            .await?
            .ok_or_else(|| AppError::NotFound("Provider not found".to_string()))?;

        Ok(ProviderResponse {
            id: provider.id,
            name: provider.name,
            r#type: provider.r#type,
            base_url: provider.base_url,
        })
    }

    pub async fn update_provider(
        &self,
        id: Uuid,
        name: &str,
        r#type: &str,
        api_key: &str,
        base_url: Option<&str>,
    ) -> Result<ProviderResponse> {
        if name.is_empty() {
            return Err(AppError::Validation("Name is required".to_string()));
        }

        let provider =
            db::update_provider(self.db.get_pool(), id, name, r#type, api_key, base_url, &self.crypto)
                .await?;

        Ok(ProviderResponse {
            id: provider.id,
            name: provider.name,
            r#type: provider.r#type,
            base_url: provider.base_url,
        })
    }

    pub async fn delete_provider(&self, id: Uuid) -> Result<()> {
        let rows = db::delete_provider(self.db.get_pool(), id).await?;

        if rows == 0 {
            return Err(AppError::NotFound("Provider not found".to_string()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_response() {
        let resp = ProviderResponse {
            id: Uuid::new_v4(),
            name: "OpenAI".to_string(),
            r#type: "openai".to_string(),
            base_url: Some("https://api.openai.com".to_string()),
        };

        assert_eq!(resp.name, "OpenAI");
        assert!(resp.base_url.is_some());
    }
}
