use sqlx::SqliteConnection;

use crate::error::AppError;

pub mod config;
pub mod recipes;
pub mod schedule;
pub mod settings;

pub use shared_core::ids::{config_id, diet_id, slot_id, tag_id, tag_link_id};

/// The current time in the `updated_at` format (ISO8601 UTC, milliseconds). SQLite produces it
/// so that it matches the column defaults in the migrations exactly.
pub async fn now(conn: &mut SqliteConnection) -> Result<String, AppError> {
    let now = sqlx::query_scalar("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')")
        .fetch_one(conn)
        .await?;
    Ok(now)
}
