use super::db;
use crate::error::{AppError, Result};
use sqlx::PgPool;

pub const BASIC_USER_NAME: &str = "basic_user";

pub async fn ensure_basic_user(pool: &PgPool) -> Result<db::User> {
    match db::get_user_by_username(pool, BASIC_USER_NAME).await {
        Ok(Some(user)) => Ok(user),
        Ok(None) => {
            tracing::info!("Creating basic_user");
            db::create_user(pool, BASIC_USER_NAME, Some("basic@agent-builder.local"))
                .await
                .map_err(AppError::Database)
        }
        Err(e) => {
            tracing::error!("Database error checking for basic_user: {}", e);
            Err(AppError::Database(e))
        }
    }
}

pub async fn get_user_by_id(pool: &PgPool, id: uuid::Uuid) -> Result<db::User> {
    db::get_user_by_id(pool, id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_user_constant() {
        assert_eq!(BASIC_USER_NAME, "basic_user");
    }
}
