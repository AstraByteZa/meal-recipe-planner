//! Sync support. `outbox` records local changes for the worker to push, `apply` writes pulled
//! changes into the local tables, and `worker` runs the push/pull/stream loop. `account` holds the
//! commands that sign in and manage families.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sqlx::SqliteConnection;
use uuid::Uuid;

use crate::error::AppError;

pub mod account;
pub mod apply;
pub mod client;
pub mod family;
pub mod outbox;
pub mod worker;

/// Set once the device is linked to an account. Until then it keeps no outbox.
pub const USER_ID: &str = "user_id";

#[cfg(test)]
mod e2e;

/// The family that rows created on this device join, or `None` if the device is not in one.
/// Existing rows keep their scope, so sharing old private data is a separate step.
pub async fn active_family(conn: &mut SqliteConnection) -> Result<Option<String>, AppError> {
    meta(conn, "active_family_id").await
}

pub async fn meta(conn: &mut SqliteConnection, key: &str) -> Result<Option<String>, AppError> {
    let value = sqlx::query_scalar("SELECT value FROM sync_meta WHERE key = ?")
        .bind(key)
        .fetch_optional(conn)
        .await?;
    Ok(value)
}

pub async fn set_meta(conn: &mut SqliteConnection, key: &str, value: &str) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO sync_meta (key, value) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn clear_meta(conn: &mut SqliteConnection, key: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM sync_meta WHERE key = ?")
        .bind(key)
        .execute(conn)
        .await?;
    Ok(())
}

/// This device's id, created on first use. The server echoes it on every op, so the device can
/// skip its own changes when they come back in a pull.
pub async fn client_id(conn: &mut SqliteConnection) -> Result<String, AppError> {
    if let Some(id) = meta(conn, "client_id").await? {
        return Ok(id);
    }
    let id = Uuid::new_v4().to_string();
    set_meta(conn, "client_id", &id).await?;
    Ok(id)
}

/// Shared between the commands and the worker. Commands call `wake` after a write or a sign-in,
/// and the worker decides what to do. The status is what `sync_status` reports.
#[derive(Clone, Default)]
pub struct SyncHandle(Arc<Shared>);

#[derive(Default)]
struct Shared {
    wake: tokio::sync::Notify,
    restart: AtomicBool,
    status: Mutex<Status>,
}

#[derive(Default)]
struct Status {
    syncing: bool,
    last_synced_at: Option<String>,
    last_error: Option<String>,
    /// `unauthorized`, `unreachable` or `other`. See `SyncError::kind`.
    last_error_kind: Option<&'static str>,
}

/// A copy of the worker's status for the UI.
pub struct StatusSnapshot {
    pub syncing: bool,
    pub last_synced_at: Option<String>,
    pub last_error: Option<String>,
    pub last_error_kind: Option<&'static str>,
}

impl SyncHandle {
    /// Asks the worker to push and pull soon.
    pub fn wake(&self) {
        self.0.wake.notify_one();
    }

    /// Asks the worker to drop its session and reload it, for example after a sign-in change or
    /// a family change, since the stream subscribes to families when it connects.
    pub fn restart(&self) {
        self.0.restart.store(true, Ordering::SeqCst);
        self.wake();
    }

    pub fn take_restart(&self) -> bool {
        self.0.restart.swap(false, Ordering::SeqCst)
    }

    pub async fn woken(&self) {
        self.0.wake.notified().await;
    }

    pub fn set_syncing(&self, syncing: bool) {
        self.status().syncing = syncing;
    }

    pub fn set_synced(&self, at: String) {
        let mut status = self.status();
        status.last_synced_at = Some(at);
        status.last_error = None;
        status.last_error_kind = None;
    }

    pub fn set_error(&self, kind: &'static str, message: String) {
        let mut status = self.status();
        status.last_error = Some(message);
        status.last_error_kind = Some(kind);
    }

    pub fn clear_error(&self) {
        let mut status = self.status();
        status.last_error = None;
        status.last_error_kind = None;
    }

    pub fn snapshot(&self) -> StatusSnapshot {
        let status = self.status();
        StatusSnapshot {
            syncing: status.syncing,
            last_synced_at: status.last_synced_at.clone(),
            last_error: status.last_error.clone(),
            last_error_kind: status.last_error_kind,
        }
    }

    fn status(&self) -> std::sync::MutexGuard<'_, Status> {
        // A poisoned lock only means a panic happened while it was held. The data is still usable.
        self.0.status.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Waits for `handle` to be woken or for `duration` to pass, whichever comes first.
pub async fn wait_or_timeout(handle: &SyncHandle, duration: Duration) {
    tokio::select! {
        _ = handle.woken() => {}
        _ = tokio::time::sleep(duration) => {}
    }
}
