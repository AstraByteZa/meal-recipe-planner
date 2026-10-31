//! Sign-in, sign-out, families and the status the sync modal shows. Tokens and server settings
//! live in `sync_meta`, so they survive restarts.

use serde::Serialize;
use sqlx::{SqliteConnection, SqlitePool};

use shared_core::entity::{EntityType, Operation};

use crate::error::AppError;
use crate::sync::client::{Api, AuthResponse, FamilyDto};
use crate::sync::outbox::{self, record};
use crate::sync::worker::LAST_SEQ_KEY;
use crate::sync::{SyncHandle, active_family, clear_meta, client_id, meta, set_meta};

const SERVER_URL: &str = "server_url";
const TOKEN: &str = "token";
const USER_ID: &str = "user_id";
const EMAIL: &str = "email";
const FAMILY_NAME: &str = "active_family_name";
const FAMILY_INVITE: &str = "active_family_invite_code";

/// Every synced table and the entity it sends as.
const SYNCED_TABLES: [(&str, EntityType); 8] = [
    ("tracks", EntityType::Track),
    ("tags", EntityType::Tag),
    ("recipes", EntityType::Recipe),
    ("recipe_tags", EntityType::RecipeTag),
    ("schedule_entries", EntityType::ScheduleEntry),
    ("settings", EntityType::Settings),
    ("household_diets", EntityType::HouseholdDiet),
    ("config", EntityType::Config),
];

/// What the worker needs to talk to the server.
pub struct Session {
    pub server_url: String,
    pub token: String,
    pub client_id: String,
}

/// The sync status the UI shows. Field names are camelCase on the wire to match `SyncStatus` in
/// the frontend schema.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub signed_in: bool,
    pub server_url: Option<String>,
    pub email: Option<String>,
    pub family: Option<FamilyInfo>,
    pub pending: i64,
    pub syncing: bool,
    pub last_synced_at: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FamilyInfo {
    pub id: String,
    pub name: String,
    pub invite_code: String,
}

/// The session the worker runs with, or `None` while signed out.
pub async fn session(pool: &SqlitePool) -> Result<Option<Session>, AppError> {
    let mut conn = pool.acquire().await?;
    let (Some(server_url), Some(token)) = (meta(&mut conn, SERVER_URL).await?, meta(&mut conn, TOKEN).await?) else {
        return Ok(None);
    };
    let client_id = client_id(&mut conn).await?;
    Ok(Some(Session { server_url, token, client_id }))
}

pub async fn status(pool: &SqlitePool, handle: &SyncHandle) -> Result<SyncStatus, AppError> {
    // Read before taking a connection: the count uses the pool, and a pool of one would deadlock.
    let pending = outbox::pending_count(pool).await?;
    let mut conn = pool.acquire().await?;
    let token = meta(&mut conn, TOKEN).await?;
    let family = match (active_family(&mut conn).await?, meta(&mut conn, FAMILY_NAME).await?, meta(&mut conn, FAMILY_INVITE).await?) {
        (Some(id), Some(name), Some(invite_code)) => Some(FamilyInfo { id, name, invite_code }),
        _ => None,
    };
    let snapshot = handle.snapshot();
    Ok(SyncStatus {
        signed_in: token.is_some(),
        server_url: meta(&mut conn, SERVER_URL).await?,
        email: if token.is_some() { meta(&mut conn, EMAIL).await? } else { None },
        family,
        pending,
        syncing: snapshot.syncing,
        last_synced_at: snapshot.last_synced_at,
        last_error: snapshot.last_error,
    })
}

pub async fn register(pool: &SqlitePool, handle: &SyncHandle, server_url: &str, email: &str, password: &str) -> Result<SyncStatus, AppError> {
    let server_url = normalize_server_url(server_url)?;
    let auth = Api::new(&server_url, None)?.register(email, password).await?;
    sign_in(pool, handle, &server_url, auth).await
}

pub async fn login(pool: &SqlitePool, handle: &SyncHandle, server_url: &str, email: &str, password: &str) -> Result<SyncStatus, AppError> {
    let server_url = normalize_server_url(server_url)?;
    let auth = Api::new(&server_url, None)?.login(email, password).await?;
    sign_in(pool, handle, &server_url, auth).await
}

/// Stores the token and claims the local rows for the account. The worker reloads its session,
/// so the first push and pull start straight away.
async fn sign_in(pool: &SqlitePool, handle: &SyncHandle, server_url: &str, auth: AuthResponse) -> Result<SyncStatus, AppError> {
    let mut tx = pool.begin().await?;
    if let Some(owner) = meta(&mut *tx, USER_ID).await? {
        if owner != auth.user.id {
            return Err(AppError::Invalid(
                "This device already holds data from another account, so it cannot sign in to this one.".to_string(),
            ));
        }
    }
    adopt_local_rows(&mut *tx, &auth.user.id).await?;
    set_meta(&mut *tx, SERVER_URL, server_url).await?;
    set_meta(&mut *tx, TOKEN, &auth.token).await?;
    set_meta(&mut *tx, USER_ID, &auth.user.id).await?;
    set_meta(&mut *tx, EMAIL, &auth.user.email).await?;
    tx.commit().await?;

    handle.clear_error();
    handle.restart();
    status(pool, handle).await
}

/// Gives every row that is still local (`owner_id = 'local'`, from before sign-in) to the
/// account, and queues it. Rows keep their timestamps, so the server settles any conflict with
/// rows that other devices already pushed. Ids are deterministic, so the same tag or track from two
/// devices becomes one row.
pub async fn adopt_local_rows(conn: &mut SqliteConnection, user_id: &str) -> Result<(), AppError> {
    for (table, entity) in SYNCED_TABLES {
        let rows: Vec<(String, Option<String>)> =
            sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT id, deleted_at FROM {table} WHERE owner_id = 'local'")))
                .fetch_all(&mut *conn)
                .await?;
        if rows.is_empty() {
            continue;
        }
        sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE {table} SET owner_id = ? WHERE owner_id = 'local'")))
            .bind(user_id)
            .execute(&mut *conn)
            .await?;
        for (id, deleted_at) in rows {
            let operation = if deleted_at.is_some() { Operation::Delete } else { Operation::Upsert };
            record(&mut *conn, entity, &id, operation).await?;
        }
    }
    Ok(())
}

/// Forgets the token and stops the worker. Local data and the account link stay, so signing back
/// in to the same account picks up where it left off.
pub async fn logout(pool: &SqlitePool, handle: &SyncHandle) -> Result<SyncStatus, AppError> {
    let mut tx = pool.begin().await?;
    clear_meta(&mut *tx, TOKEN).await?;
    clear_meta(&mut *tx, EMAIL).await?;
    tx.commit().await?;
    handle.restart();
    status(pool, handle).await
}

pub async fn family_create(pool: &SqlitePool, handle: &SyncHandle, name: &str) -> Result<SyncStatus, AppError> {
    let session = require_session(pool).await?;
    let api = Api::new(&session.server_url, Some(&session.token))?;
    let family = api.create_family(name).await?;
    activate_family(pool, handle, &family, false).await
}

pub async fn family_join(pool: &SqlitePool, handle: &SyncHandle, invite_code: &str) -> Result<SyncStatus, AppError> {
    let session = require_session(pool).await?;
    let api = Api::new(&session.server_url, Some(&session.token))?;
    let family = api.join_family(invite_code.trim()).await?;
    activate_family(pool, handle, &family, true).await
}

/// Makes `family` the one that new rows join. A join also resets the pull cursor to 0. The
/// server only sends a family's history to members who pull, and a member who has already
/// passed it would never see it. Pulls are idempotent under LWW, so re-pulling is safe.
async fn activate_family(pool: &SqlitePool, handle: &SyncHandle, family: &FamilyDto, rewind: bool) -> Result<SyncStatus, AppError> {
    let mut tx = pool.begin().await?;
    set_meta(&mut *tx, "active_family_id", &family.id).await?;
    set_meta(&mut *tx, FAMILY_NAME, &family.name).await?;
    set_meta(&mut *tx, FAMILY_INVITE, &family.invite_code).await?;
    if rewind {
        set_meta(&mut *tx, LAST_SEQ_KEY, "0").await?;
    }
    tx.commit().await?;
    handle.restart();
    status(pool, handle).await
}

async fn require_session(pool: &SqlitePool) -> Result<Session, AppError> {
    session(pool)
        .await?
        .ok_or_else(|| AppError::Invalid("Sign in to sync before joining or creating a family.".to_string()))
}

/// Checks and tidies the server address: http or https, no trailing slash.
pub fn normalize_server_url(raw: &str) -> Result<String, AppError> {
    let url = raw.trim().trim_end_matches('/');
    if url.starts_with("https://") || url.starts_with("http://") {
        Ok(url.to_string())
    } else {
        Err(AppError::Invalid("The server address must start with http:// or https://.".to_string()))
    }
}

/// Wakes the worker for a manual sync. The status returned is the state before the sync finishes.
pub async fn sync_now(pool: &SqlitePool, handle: &SyncHandle) -> Result<SyncStatus, AppError> {
    handle.wake();
    status(pool, handle).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_pool;
    use crate::models::RecipeInput;
    use crate::repo::recipes;

    fn recipe(name: &str) -> RecipeInput {
        RecipeInput {
            name: name.to_string(),
            url: String::new(),
            source_domain: String::new(),
            minutes: None,
            rating: None,
            servings: None,
            notes: None,
            usual_track_key: None,
            tags: vec![],
            ingredients: vec![],
            instructions: vec![],
            image_url: None,
        }
    }

    #[test]
    fn server_addresses_must_be_http_and_lose_the_trailing_slash() {
        assert_eq!(normalize_server_url(" https://sync.example.com/ ").unwrap(), "https://sync.example.com");
        assert_eq!(normalize_server_url("http://10.0.0.5:8080").unwrap(), "http://10.0.0.5:8080");
        assert!(normalize_server_url("sync.example.com").is_err());
        assert!(normalize_server_url("ftp://example.com").is_err());
    }

    #[test]
    fn adopting_queues_every_local_row_under_the_account() {
        tauri::async_runtime::block_on(async {
            let pool = test_pool().await;
            recipes::create(&pool, &recipe("oats")).await.unwrap();
            sqlx::query("DELETE FROM outbox_queue").execute(&pool).await.unwrap();

            let mut conn = pool.acquire().await.unwrap();
            adopt_local_rows(&mut conn, "user-1").await.unwrap();
            drop(conn);

            let owners: Vec<String> = sqlx::query_scalar("SELECT DISTINCT owner_id FROM recipes")
                .fetch_all(&pool)
                .await
                .unwrap();
            assert_eq!(owners, ["user-1"]);

            let queued: Vec<(String, String)> =
                sqlx::query_as("SELECT entity_type, payload FROM outbox_queue ORDER BY id")
                    .fetch_all(&pool)
                    .await
                    .unwrap();
            let entities: Vec<&str> = queued.iter().map(|(entity, _)| entity.as_str()).collect();
            // The four seeded tracks are local rows too, so they are queued with the recipe.
            assert!(entities.contains(&"recipe"));
            assert_eq!(entities.iter().filter(|entity| **entity == "track").count(), 4);
            assert!(queued.iter().all(|(_, payload)| payload.contains("\"owner_id\":\"user-1\"")));
        });
    }

    #[test]
    fn adopting_leaves_rows_that_already_belong_to_an_account_alone() {
        tauri::async_runtime::block_on(async {
            let pool = test_pool().await;
            recipes::create(&pool, &recipe("oats")).await.unwrap();
            let mut conn = pool.acquire().await.unwrap();
            adopt_local_rows(&mut conn, "user-1").await.unwrap();
            drop(conn);
            sqlx::query("DELETE FROM outbox_queue").execute(&pool).await.unwrap();

            let mut conn = pool.acquire().await.unwrap();
            adopt_local_rows(&mut conn, "user-2").await.unwrap();
            drop(conn);

            assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM outbox_queue").fetch_one(&pool).await.unwrap(), 0);
            let owner: String = sqlx::query_scalar("SELECT owner_id FROM recipes").fetch_one(&pool).await.unwrap();
            assert_eq!(owner, "user-1");
        });
    }

    #[test]
    fn a_device_without_a_session_reports_signed_out() {
        tauri::async_runtime::block_on(async {
            let pool = test_pool().await;
            assert!(session(&pool).await.unwrap().is_none());
            let status = status(&pool, &SyncHandle::default()).await.unwrap();
            assert!(!status.signed_in);
            assert_eq!(status.pending, 0);
            assert!(status.family.is_none());
        });
    }
}
