use super::db;
use crate::db::Database;
use crate::error::{AppError, Result};

pub const BASIC_USER_NAME: &str = "basic_user";

#[derive(Clone)]
pub struct UserService {
    pub db: Database,
}

impl UserService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn ensure_basic_user(&self) -> Result<db::User> {
        match db::get_user_by_username(self.db.get_pool(), BASIC_USER_NAME).await {
            Ok(Some(user)) => Ok(user),
            Ok(None) => {
                tracing::info!("Creating basic_user");
                db::create_user(
                    self.db.get_pool(),
                    BASIC_USER_NAME,
                    Some("basic@agent-builder.local"),
                )
                .await
                .map_err(AppError::Database)
            }
            Err(e) => {
                tracing::error!("Database error checking for basic_user: {}", e);
                Err(AppError::Database(e))
            }
        }
    }

    pub async fn get_user_by_id(&self, id: uuid::Uuid) -> Result<db::User> {
        db::get_user_by_id(self.db.get_pool(), id)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_user_constant() {
        assert_eq!(BASIC_USER_NAME, "basic_user");
    }
}
