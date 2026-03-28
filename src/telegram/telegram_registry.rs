use uuid::Uuid;

use crate::db::Database;
use crate::error::{AppError, Result};
use crate::telegram::db::TelegramConfig;

/// Postgres-backed distributed lease + offset registry.
///
/// Each backend process has a unique `instance_id`. Connectors are claimed via a
/// short atomic UPDATE. Heartbeats renew the 30-second lease every 10 seconds.
/// Offsets are persisted BEFORE processing each batch (crash-safe).
pub struct TelegramRegistry {
    db: Database,
    pub instance_id: String,
}

impl TelegramRegistry {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            instance_id: Uuid::new_v4().to_string(),
        }
    }

    /// Returns all connectors that are enabled and whose lease has expired or is unclaimed.
    pub async fn list_claimable(&self) -> Result<Vec<TelegramConfig>> {
        sqlx::query_as::<_, TelegramConfig>(
            r#"
            SELECT * FROM telegram_configs
            WHERE is_enabled = true
              AND (lease_expires_at IS NULL OR lease_expires_at < NOW())
            "#,
        )
        .fetch_all(self.db.get_pool())
        .await
        .map_err(AppError::Database)
    }

    /// Atomically claims one connector. Returns true if this instance won the claim,
    /// false if another instance beat us to it.
    pub async fn try_claim(&self, connector_id: Uuid) -> Result<bool> {
        let rows = sqlx::query(
            r#"
            UPDATE telegram_configs
            SET owner_instance_id = $1,
                lease_expires_at  = NOW() + INTERVAL '30 seconds',
                updated_at        = NOW()
            WHERE id = $2
              AND (lease_expires_at IS NULL OR lease_expires_at < NOW())
            "#,
        )
        .bind(&self.instance_id)
        .bind(connector_id)
        .execute(self.db.get_pool())
        .await
        .map_err(AppError::Database)?
        .rows_affected();

        Ok(rows > 0)
    }

    /// Extends the lease by 30 seconds. Called every ~10 s by each active poller.
    pub async fn heartbeat(&self, connector_id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE telegram_configs
            SET lease_expires_at = NOW() + INTERVAL '30 seconds',
                updated_at       = NOW()
            WHERE id = $1 AND owner_instance_id = $2
            "#,
        )
        .bind(connector_id)
        .bind(&self.instance_id)
        .execute(self.db.get_pool())
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    /// Persists the polling offset. Called BEFORE processing each batch so that
    /// a crash always results in safe re-delivery rather than data loss.
    pub async fn save_offset(&self, connector_id: Uuid, offset: i64) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE telegram_configs
            SET last_update_id = $1, updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(offset)
        .bind(connector_id)
        .execute(self.db.get_pool())
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    /// Releases the lease on clean shutdown.
    pub async fn release(&self, connector_id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE telegram_configs
            SET owner_instance_id = NULL,
                lease_expires_at  = NULL,
                updated_at        = NOW()
            WHERE id = $1 AND owner_instance_id = $2
            "#,
        )
        .bind(connector_id)
        .bind(&self.instance_id)
        .execute(self.db.get_pool())
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    /// Returns the persisted polling offset for a connector (used on claim/resume).
    pub async fn get_offset(&self, connector_id: Uuid) -> Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT last_update_id FROM telegram_configs WHERE id = $1")
                .bind(connector_id)
                .fetch_one(self.db.get_pool())
                .await
                .map_err(AppError::Database)?;

        Ok(row.0)
    }
}
