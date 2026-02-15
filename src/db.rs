use crate::config::DbConfig;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;

#[derive(Clone)]
pub struct Database {
    pub pool: PgPool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub database: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Database {
    pub async fn new(config: &DbConfig) -> Result<Self, sqlx::Error> {
        let database_url = format!(
            "postgres://{}:{}@{}:{}/{}",
            config.user, config.password, config.host, config.port, config.dbname
        );

        let pool = PgPoolOptions::new()
            .max_connections(20)
            .acquire_timeout(Duration::from_secs(5))
            .connect(&database_url)
            .await?;

        sqlx::query("SELECT 1")
            .execute(&pool)
            .await?;

        Ok(Database { pool })
    }

    pub async fn health_check(&self) -> HealthStatus {
        match sqlx::query("SELECT 1").execute(&self.pool).await {
            Ok(_) => HealthStatus {
                status: "healthy".to_string(),
                database: "connected".to_string(),
                error: None,
            },
            Err(e) => HealthStatus {
                status: "unhealthy".to_string(),
                database: "disconnected".to_string(),
                error: Some(e.to_string()),
            },
        }
    }

    pub fn get_pool(&self) -> &PgPool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_healthy() {
        let status = HealthStatus {
            status: "healthy".to_string(),
            database: "connected".to_string(),
            error: None,
        };
        assert_eq!(status.status, "healthy");
        assert_eq!(status.database, "connected");
        assert!(status.error.is_none());
    }

    #[test]
    fn test_health_status_unhealthy() {
        let status = HealthStatus {
            status: "unhealthy".to_string(),
            database: "disconnected".to_string(),
            error: Some("Connection timeout".to_string()),
        };
        assert_eq!(status.status, "unhealthy");
        assert!(status.error.is_some());
    }
}
