//! The background loop. It pushes queued changes, pulls the server log from the saved cursor, and
//! keeps a `/sync/stream` connection open so it pulls as soon as another device writes. Failures
//! back off exponentially, and the loop waits quietly while the device is signed out.

use std::time::Duration;

use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use shared_core::entity::{EntityType, Operation};
use shared_core::wire::{OpStatus, PushRequest, SyncOp};

use crate::error::AppError;
use crate::repo;
use crate::sync::account::{self, Session};
use crate::sync::apply;
use crate::sync::client::{Api, SyncError};
use crate::sync::outbox::{self, Pending};
use crate::sync::{SyncHandle, meta, set_meta, wait_or_timeout};

/// Emitted after pulled changes land locally. The frontend invalidates its queries on it.
pub const CHANGED_EVENT: &str = "sync://changed";
pub const LAST_SEQ_KEY: &str = "last_seq";

const PUSH_BATCH: i64 = 100;
const PULL_PAGE: i64 = 500;
/// Local edits often come in bursts (a drag, or several slots in a row). Wait a moment so they go
/// in one push.
const DEBOUNCE: Duration = Duration::from_millis(250);
const FIRST_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(60);

pub fn spawn(app: AppHandle, pool: SqlitePool, handle: SyncHandle) {
    tauri::async_runtime::spawn(run(app, pool, handle));
}

async fn run(app: AppHandle, pool: SqlitePool, handle: SyncHandle) {
    let mut backoff = FIRST_BACKOFF;
    loop {
        // A restart requested before this point is already covered by the session load below.
        handle.take_restart();
        let session = match account::session(&pool).await {
            Ok(Some(session)) => session,
            Ok(None) => {
                handle.woken().await;
                continue;
            }
            Err(error) => {
                handle.set_error(error.to_string());
                wait_or_timeout(&handle, backoff).await;
                backoff = next_backoff(backoff);
                continue;
            }
        };

        match session_loop(&app, &pool, &handle, &session, &mut backoff).await {
            // A restart or sign-out ends the session cleanly.
            Ok(()) => {}
            Err(SyncError::Unauthorized(message)) => {
                handle.set_error(message);
                handle.woken().await;
            }
            Err(error) => {
                handle.set_error(error.to_string());
                wait_or_timeout(&handle, backoff).await;
                backoff = next_backoff(backoff);
            }
        }
    }
}

fn next_backoff(current: Duration) -> Duration {
    (current * 2).min(MAX_BACKOFF)
}

/// Runs one signed-in session: an initial sync, then a stream connection that pulls on each event
/// and pushes on each local wake. Returns Ok only when a restart was requested.
async fn session_loop(
    app: &AppHandle,
    pool: &SqlitePool,
    handle: &SyncHandle,
    session: &Session,
    backoff: &mut Duration,
) -> Result<(), SyncError> {
    let api = Api::new(&session.server_url, Some(&session.token))?;
    outbox::requeue_in_flight(pool).await?;

    loop {
        sync_once(app, pool, handle, &api, session).await?;
        *backoff = FIRST_BACKOFF;

        let mut stream = api.stream().await?;
        loop {
            tokio::select! {
                _ = handle.woken() => {
                    if handle.take_restart() {
                        return Ok(());
                    }
                    tokio::time::sleep(DEBOUNCE).await;
                    sync_once(app, pool, handle, &api, session).await?;
                }
                event = stream.next_event() => {
                    event?;
                    sync_once(app, pool, handle, &api, session).await?;
                }
            }
        }
    }
}

async fn sync_once(
    app: &AppHandle,
    pool: &SqlitePool,
    handle: &SyncHandle,
    api: &Api,
    session: &Session,
) -> Result<(), SyncError> {
    handle.set_syncing(true);
    let result = push_then_pull(pool, api, session).await;
    handle.set_syncing(false);
    let changed = result?;

    if changed {
        // The emit only fails if no window is listening, which is fine.
        let _ = app.emit(CHANGED_EVENT, ());
    }
    let mut conn = pool.acquire().await.map_err(AppError::from)?;
    handle.set_synced(repo::now(&mut *conn).await?);
    Ok(())
}

async fn push_then_pull(pool: &SqlitePool, api: &Api, session: &Session) -> Result<bool, SyncError> {
    push_all(pool, api, &session.client_id).await?;
    pull_all(pool, api, &session.client_id).await
}

async fn push_all(pool: &SqlitePool, api: &Api, client_id: &str) -> Result<(), SyncError> {
    loop {
        let batch = outbox::take_batch(pool, PUSH_BATCH).await?;
        if batch.is_empty() {
            return Ok(());
        }

        let mut ids = Vec::new();
        let mut ops = Vec::new();
        let mut unsendable = Vec::new();
        for row in &batch {
            match to_op(row) {
                Ok(op) => {
                    ids.push(row.id);
                    ops.push(op);
                }
                Err(reason) => {
                    eprintln!("sync: not pushing outbox row {}: {reason}", row.id);
                    unsendable.push(row.id);
                }
            }
        }
        if !unsendable.is_empty() {
            outbox::mark_failed(pool, &unsendable).await?;
        }
        if ops.is_empty() {
            continue;
        }

        let request = PushRequest { client_id: client_id.to_string(), ops };
        match api.push(&request).await {
            Ok(response) if response.results.len() == ids.len() => {
                for result in response.results.iter().filter(|result| result.status == OpStatus::Rejected) {
                    eprintln!("sync: server rejected a change to {}: {}", result.entity_id, result.reason.as_deref().unwrap_or("no reason given"));
                }
                // Applied, stale and rejected all mean the server has decided, so the row is done.
                outbox::finish(pool, &ids).await?;
            }
            Ok(_) => {
                outbox::requeue_in_flight(pool).await?;
                return Err(SyncError::Unreachable("push response did not match the batch".to_string()));
            }
            Err(SyncError::Rejected(message)) => {
                eprintln!("sync: push refused: {message}");
                outbox::mark_failed(pool, &ids).await?;
            }
            Err(error) => {
                outbox::requeue_in_flight(pool).await?;
                return Err(error);
            }
        }
    }
}

/// Pulls from the saved cursor until the server has nothing more. Each page and its new cursor
/// are saved in one transaction, so a crash never skips ops.
async fn pull_all(pool: &SqlitePool, api: &Api, client_id: &str) -> Result<bool, SyncError> {
    let mut changed = false;
    loop {
        let since = last_seq(pool).await?;
        let page = api.pull(since, PULL_PAGE).await?;

        let mut tx = pool.begin().await?;
        changed |= apply::apply_page(&mut *tx, client_id, &page.ops).await?;
        set_meta(&mut *tx, LAST_SEQ_KEY, &page.last_seq.to_string()).await?;
        tx.commit().await?;

        if !page.has_more {
            return Ok(changed);
        }
    }
}

async fn last_seq(pool: &SqlitePool) -> Result<i64, AppError> {
    let mut conn = pool.acquire().await?;
    let seq = meta(&mut *conn, LAST_SEQ_KEY).await?;
    Ok(seq.and_then(|seq| seq.parse().ok()).unwrap_or(0))
}

/// Builds the wire op for a queued row. An error means the row can never be sent.
fn to_op(row: &Pending) -> Result<SyncOp, String> {
    let entity_type: EntityType = serde_json::from_value(serde_json::Value::String(row.entity_type.clone()))
        .map_err(|_| format!("unknown entity type {}", row.entity_type))?;
    let op: Operation = serde_json::from_value(serde_json::Value::String(row.operation.clone()))
        .map_err(|_| format!("unknown operation {}", row.operation))?;
    let entity_id = Uuid::parse_str(&row.entity_id).map_err(|_| format!("bad entity id {}", row.entity_id))?;
    let payload: serde_json::Value =
        serde_json::from_str(&row.payload).map_err(|_| "payload is not JSON".to_string())?;
    let family_id = match payload["family_id"].as_str() {
        Some(family) => Some(Uuid::parse_str(family).map_err(|_| "bad family id".to_string())?),
        None => None,
    };
    Ok(SyncOp {
        entity_type,
        entity_id,
        op,
        family_id,
        client_timestamp: row.client_timestamp.clone(),
        payload,
    })
}
