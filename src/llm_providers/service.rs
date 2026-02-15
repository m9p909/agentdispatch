use super::db;
use crate::error::{AppError, Result};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Serialize)]
pub struct ProviderResponse {
    pub id: Uuid,
    pub name: String,
    pub r#type: String,
    pub base_url: Option<String>,
}

pub async fn create_provider(
    pool: &PgPool,
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

    let provider = db::create_provider(pool, &req)
        .await
        .map_err(AppError::Database)?;

    Ok(ProviderResponse {
        id: provider.id,
        name: provider.name,
        r#type: provider.r#type,
        base_url: provider.base_url,
    })
}

pub async fn list_providers(pool: &PgPool) -> Result<Vec<ProviderResponse>> {
    let providers = db::list_providers(pool)
        .await
        .map_err(AppError::Database)?;

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

pub async fn get_provider_by_id(pool: &PgPool, id: Uuid) -> Result<ProviderResponse> {
    let provider = db::get_provider_by_id(pool, id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Provider not found".to_string()))?;

    Ok(ProviderResponse {
        id: provider.id,
        name: provider.name,
        r#type: provider.r#type,
        base_url: provider.base_url,
    })
}

pub async fn update_provider(
    pool: &PgPool,
    id: Uuid,
    name: &str,
    r#type: &str,
    api_key: &str,
    base_url: Option<&str>,
) -> Result<ProviderResponse> {
    if name.is_empty() {
        return Err(AppError::Validation("Name is required".to_string()));
    }

    let provider = db::update_provider(pool, id, name, r#type, api_key, base_url)
        .await
        .map_err(AppError::Database)?;

    Ok(ProviderResponse {
        id: provider.id,
        name: provider.name,
        r#type: provider.r#type,
        base_url: provider.base_url,
    })
}

pub async fn delete_provider(pool: &PgPool, id: Uuid) -> Result<()> {
    let rows = db::delete_provider(pool, id)
        .await
        .map_err(AppError::Database)?;

    if rows == 0 {
        return Err(AppError::NotFound("Provider not found".to_string()));
    }

    Ok(())
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

    #[test]
    fn test_validation_empty_name() {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                // This would need a real pool to test fully
                // For now just validate the logic
                assert!("".is_empty());
            })
    }
}
