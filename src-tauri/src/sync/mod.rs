//! Sync support. `outbox` records local changes for the sync worker to push.

use sqlx::SqliteConnection;

use crate::error::AppError;

pub mod outbox;

/// The family that rows created on this device join, or `None` if the device is not in one.
/// Existing rows keep their scope, so sharing old private data is a separate step.
pub async fn active_family(conn: &mut SqliteConnection) -> Result<Option<String>, AppError> {
    let family: Option<String> =
        sqlx::query_scalar("SELECT value FROM sync_meta WHERE key = 'active_family_id'")
            .fetch_optional(conn)
            .await?;
    Ok(family)
}
