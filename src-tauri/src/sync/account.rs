//! Sign-in, sign-out, families and the status the sync modal shows. Tokens and server settings
//! live in `sync_meta`, so they survive restarts.
//!
//! A device is in one of three account modes. `local` has never signed in. `signed_out` is linked
//! to an account but has no token, so it stops syncing and keeps its link. `signed_in` has a
//! token and syncs. Changes made while `local` are not queued. Signing in queues them all.

use serde::{Deserialize, Serialize};
use sqlx::{SqliteConnection, SqlitePool};

use shared_core::entity::{EntityType, Operation};
use shared_core::wire::FamilyRole;

use crate::error::AppError;
use crate::sync::client::{Api, AuthResponse, FamilyDto};
use crate::sync::outbox::{self, enqueue};
use crate::sync::worker::LAST_SEQ_KEY;
use crate::sync::{SyncHandle, USER_ID, clear_meta, client_id, meta, set_meta};

/// The server address built into the app, from `PLANNER_SYNC_SERVER_URL` at compile time. It
/// pre-fills the server field. A server address the user saved wins over it.
pub const DEFAULT_SERVER_URL: Option<&str> = option_env!("PLANNER_SYNC_SERVER_URL");

const SERVER_URL: &str = "server_url";
const TOKEN: &str = "token";
const EMAIL: &str = "email";
const DISPLAY_NAME: &str = "display_name";
pub(crate) const FAMILY_ID: &str = "active_family_id";
pub(crate) const FAMILY_NAME: &str = "active_family_name";
pub(crate) const FAMILY_ROLE: &str = "active_family_role";
/// Invite codes used to be kept on the device. Only this key is still cleared.
pub(crate) const FAMILY_INVITE: &str = "active_family_invite_code";
/// Set when this device was removed from a family, as JSON. Kept until the user dismisses it.
pub(crate) const FAMILY_NOTICE: &str = "family_notice";

/// Every synced table and the entity it sends as.
pub(crate) const SYNCED_TABLES: [(&str, EntityType); 8] = [
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountMode {
    /// Never signed in. Everything stays on this device.
    Local,
    /// Linked to an account, but signed out. Nothing syncs until the user signs in again.
    SignedOut,
    /// Signed in and syncing.
    SignedIn,
}

/// The sync status the UI shows. Field names are camelCase on the wire to match `SyncStatus` in
/// the frontend schema.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub mode: AccountMode,
    /// True only in `signed_in` mode. Kept so older callers can still read it.
    pub signed_in: bool,
    pub server_url: Option<String>,
    pub default_server_url: Option<String>,
    /// Present only while signed in.
    pub user: Option<UserInfo>,
    pub family: Option<FamilyInfo>,
    /// Set after this device was removed from its family, until the user dismisses it.
    pub notice: Option<FamilyNotice>,
    /// Changes waiting to sync. Always 0 unless signed in, since nothing can sync otherwise.
    pub pending: i64,
    pub syncing: bool,
    pub last_synced_at: Option<String>,
    pub last_error: Option<String>,
    /// `unauthorized`, `unreachable` or `other`. Null when there is no error.
    pub last_error_kind: Option<&'static str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    /// The display name, or the part of the email before `@` when none is set.
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FamilyInfo {
    pub id: String,
    pub name: String,
    pub role: FamilyRole,
}

/// Why this device left its family. `kind` is `removed` for now: the server does not say whether
/// a family was deleted or the person was removed, so both read as removed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FamilyNotice {
    pub kind: String,
    pub family_name: String,
}

/// The stored settings the status is built from, read in one pass.
struct Stored {
    token: Option<String>,
    user_id: Option<String>,
    server_url: Option<String>,
    email: Option<String>,
    display_name: Option<String>,
    family: Option<FamilyInfo>,
    notice: Option<FamilyNotice>,
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
    // Read everything before the pending count, which uses the pool. A pool of one would deadlock.
    let stored = {
        let mut conn = pool.acquire().await?;
        read_stored(&mut conn).await?
    };
    let mode = account_mode(stored.token.is_some(), stored.user_id.is_some());
    let signed_in = mode == AccountMode::SignedIn;
    let pending = if signed_in { outbox::pending_count(pool).await? } else { 0 };
    let user = match (signed_in, stored.email) {
        (true, Some(email)) => Some(UserInfo {
            name: display_name_or_local_part(stored.display_name.as_deref(), &email),
            email,
        }),
        _ => None,
    };

    let snapshot = handle.snapshot();
    Ok(SyncStatus {
        mode,
        signed_in,
        server_url: stored.server_url.or_else(default_server_url),
        default_server_url: default_server_url(),
        user,
        family: stored.family,
        notice: stored.notice,
        pending,
        syncing: snapshot.syncing,
        last_synced_at: snapshot.last_synced_at,
        last_error: snapshot.last_error,
        last_error_kind: snapshot.last_error_kind,
    })
}

async fn read_stored(conn: &mut SqliteConnection) -> Result<Stored, AppError> {
    Ok(Stored {
        token: meta(conn, TOKEN).await?,
        user_id: meta(conn, USER_ID).await?,
        server_url: meta(conn, SERVER_URL).await?,
        email: meta(conn, EMAIL).await?,
        display_name: meta(conn, DISPLAY_NAME).await?,
        family: active_family_info(conn).await?,
        notice: meta(conn, FAMILY_NOTICE).await?.and_then(|json| serde_json::from_str(&json).ok()),
    })
}

/// The family this device is active in. Needs both the id and the name.
pub(crate) async fn active_family_info(conn: &mut SqliteConnection) -> Result<Option<FamilyInfo>, AppError> {
    let (Some(id), Some(name)) = (meta(conn, FAMILY_ID).await?, meta(conn, FAMILY_NAME).await?) else {
        return Ok(None);
    };
    // A family joined before roles existed has no stored role. Treat it as a member, the role
    // that can do least. The next family refresh corrects it.
    let role = meta(conn, FAMILY_ROLE).await?.as_deref().map_or(FamilyRole::Member, role_from_text);
    Ok(Some(FamilyInfo { id, name, role }))
}

/// Mode from the two facts that decide it: a token, and a linked account.
pub fn account_mode(has_token: bool, linked: bool) -> AccountMode {
    match (has_token, linked) {
        (true, _) => AccountMode::SignedIn,
        (false, true) => AccountMode::SignedOut,
        (false, false) => AccountMode::Local,
    }
}

fn default_server_url() -> Option<String> {
    DEFAULT_SERVER_URL.filter(|url| !url.is_empty()).map(str::to_string)
}

fn display_name_or_local_part(display_name: Option<&str>, email: &str) -> String {
    match display_name.map(str::trim).filter(|name| !name.is_empty()) {
        Some(name) => name.to_string(),
        None => email.split('@').next().unwrap_or(email).to_string(),
    }
}

pub async fn register(
    pool: &SqlitePool,
    handle: &SyncHandle,
    server_url: Option<&str>,
    email: &str,
    password: &str,
    display_name: &str,
) -> Result<SyncStatus, AppError> {
    let server_url = resolve_server_url(server_url, default_server_url().as_deref())?;
    let auth = Api::new(&server_url, None)?.register(email, password, display_name.trim()).await?;
    sign_in(pool, handle, &server_url, auth).await
}

pub async fn login(pool: &SqlitePool, handle: &SyncHandle, server_url: Option<&str>, email: &str, password: &str) -> Result<SyncStatus, AppError> {
    let server_url = resolve_server_url(server_url, default_server_url().as_deref())?;
    let auth = Api::new(&server_url, None)?.login(email, password).await?;
    sign_in(pool, handle, &server_url, auth).await
}

/// Stores the token and claims the local rows for the account. The worker reloads its session,
/// so the first push and pull start straight away.
pub(crate) async fn sign_in(pool: &SqlitePool, handle: &SyncHandle, server_url: &str, auth: AuthResponse) -> Result<SyncStatus, AppError> {
    let user_id = auth.user.id.to_string();
    let mut tx = pool.begin().await?;
    if let Some(owner) = meta(&mut *tx, USER_ID).await? {
        if owner != user_id {
            return Err(AppError::Invalid(
                "This device already holds data from another account, so it cannot sign in to this one.".to_string(),
            ));
        }
    }
    set_meta(&mut *tx, USER_ID, &user_id).await?;
    set_meta(&mut *tx, SERVER_URL, server_url).await?;
    set_meta(&mut *tx, TOKEN, &auth.token).await?;
    set_meta(&mut *tx, EMAIL, &auth.user.email).await?;
    set_meta(&mut *tx, DISPLAY_NAME, &auth.user.display_name).await?;
    adopt_local_rows(&mut *tx, &user_id).await?;
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
            enqueue(&mut *conn, entity, &id, operation).await?;
        }
    }
    Ok(())
}

/// Forgets the token and stops the worker. The account link, the family and local data stay, so
/// signing back in to the same account picks up where it left off. Another account cannot sign
/// in on this device until the device is returned to local.
pub async fn logout(pool: &SqlitePool, handle: &SyncHandle) -> Result<SyncStatus, AppError> {
    let mut tx = pool.begin().await?;
    clear_meta(&mut *tx, TOKEN).await?;
    clear_meta(&mut *tx, EMAIL).await?;
    clear_meta(&mut *tx, DISPLAY_NAME).await?;
    tx.commit().await?;
    handle.restart();
    status(pool, handle).await
}

/// Makes this device local again, whatever its mode. Rows stay on the device, leave any family,
/// and stop syncing. Nothing is deleted on the server. Signing in later adopts the rows again.
pub async fn reset_to_local(pool: &SqlitePool, handle: &SyncHandle) -> Result<SyncStatus, AppError> {
    let mut tx = pool.begin().await?;
    for (table, _) in SYNCED_TABLES {
        sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE {table} SET owner_id = 'local', family_id = NULL")))
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("DELETE FROM outbox_queue").execute(&mut *tx).await?;
    for key in [
        LAST_SEQ_KEY,
        USER_ID,
        EMAIL,
        DISPLAY_NAME,
        TOKEN,
        FAMILY_ID,
        FAMILY_NAME,
        FAMILY_ROLE,
        FAMILY_INVITE,
        FAMILY_NOTICE,
    ] {
        clear_meta(&mut *tx, key).await?;
    }
    tx.commit().await?;

    handle.clear_error();
    handle.restart();
    status(pool, handle).await
}

/// Changes the signed-in user's display name on the server and on this device.
pub async fn set_display_name(pool: &SqlitePool, handle: &SyncHandle, name: &str) -> Result<SyncStatus, AppError> {
    let session = require_session(pool).await?;
    let api = Api::new(&session.server_url, Some(&session.token))?;
    let user = api.update_me(name.trim()).await?;
    let mut conn = pool.acquire().await?;
    set_meta(&mut conn, DISPLAY_NAME, &user.display_name).await?;
    drop(conn);
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
    store_active(&mut *tx, family, rewind).await?;
    tx.commit().await?;
    handle.restart();
    status(pool, handle).await
}

/// Makes `family` the one that new rows join, and clears any notice about an earlier family. A
/// rewind resets the pull cursor to 0. The server only sends a family's history to members who
/// pull, so a member who has passed it would never see it. Pulls are idempotent under LWW.
pub(crate) async fn store_active(conn: &mut SqliteConnection, family: &FamilyDto, rewind: bool) -> Result<(), AppError> {
    set_meta(conn, FAMILY_ID, &family.id).await?;
    set_meta(conn, FAMILY_NAME, &family.name).await?;
    set_meta(conn, FAMILY_ROLE, role_text(family.role)).await?;
    // Invite codes are per invite now and the owner creates them, so no code is kept here.
    clear_meta(conn, FAMILY_INVITE).await?;
    clear_meta(conn, FAMILY_NOTICE).await?;
    if rewind {
        set_meta(conn, LAST_SEQ_KEY, "0").await?;
    }
    Ok(())
}

/// Hides the notice about a removed family. The user has seen it.
pub async fn dismiss_notice(pool: &SqlitePool, handle: &SyncHandle) -> Result<SyncStatus, AppError> {
    let mut conn = pool.acquire().await?;
    clear_meta(&mut conn, FAMILY_NOTICE).await?;
    drop(conn);
    status(pool, handle).await
}

pub(crate) fn role_text(role: FamilyRole) -> &'static str {
    match role {
        FamilyRole::Owner => "owner",
        FamilyRole::Member => "member",
    }
}

fn role_from_text(text: &str) -> FamilyRole {
    if text == "owner" { FamilyRole::Owner } else { FamilyRole::Member }
}

pub(crate) async fn require_session(pool: &SqlitePool) -> Result<Session, AppError> {
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

/// The address to sign in to. A typed address wins. A blank one means the built-in server, so a
/// build with no default and no typed address has nowhere to go and says so.
pub fn resolve_server_url(input: Option<&str>, default: Option<&str>) -> Result<String, AppError> {
    match input.map(str::trim).filter(|value| !value.is_empty()) {
        Some(typed) => normalize_server_url(typed),
        None => match default.map(str::trim).filter(|value| !value.is_empty()) {
            Some(built_in) => normalize_server_url(built_in),
            None => Err(AppError::Invalid(
                "No sync server is set up. Open Advanced Options and enter your server's address.".to_string(),
            )),
        },
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
    use shared_core::wire::UserDto;
    use uuid::Uuid;

    const SERVER: &str = "https://sync.example.com";

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

    fn auth(display_name: &str) -> AuthResponse {
        AuthResponse {
            token: "token-1".to_string(),
            user: UserDto { id: Uuid::new_v4(), email: "cook@example.com".to_string(), display_name: display_name.to_string() },
        }
    }

    async fn queued(pool: &SqlitePool) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM outbox_queue").fetch_one(pool).await.unwrap()
    }

    #[test]
    fn a_typed_address_wins_and_a_blank_one_uses_the_built_in_server() {
        let built_in = Some("https://sync.example.com");
        assert_eq!(resolve_server_url(Some(" http://10.0.0.5:8080/ "), built_in).unwrap(), "http://10.0.0.5:8080");
        assert_eq!(resolve_server_url(Some("   "), built_in).unwrap(), "https://sync.example.com");
        assert_eq!(resolve_server_url(None, built_in).unwrap(), "https://sync.example.com");
        assert!(resolve_server_url(None, Some("  ")).is_err());
    }

    #[test]
    fn no_address_and_no_built_in_server_is_a_clear_error() {
        let error = resolve_server_url(Some(""), None).unwrap_err();
        assert!(error.to_string().contains("No sync server is set up"), "{error}");
        assert!(resolve_server_url(Some("ftp://example.com"), Some("https://sync.example.com")).is_err());
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

            assert_eq!(queued(&pool).await, 0);
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
            assert_eq!(status.mode, AccountMode::Local);
            assert!(!status.signed_in);
            assert_eq!(status.pending, 0);
            assert!(status.family.is_none());
            assert!(status.user.is_none());
        });
    }

    #[test]
    fn mode_follows_the_token_and_the_linked_account() {
        assert_eq!(account_mode(false, false), AccountMode::Local);
        assert_eq!(account_mode(false, true), AccountMode::SignedOut);
        assert_eq!(account_mode(true, true), AccountMode::SignedIn);
        assert_eq!(account_mode(true, false), AccountMode::SignedIn);
    }

    #[test]
    fn local_changes_are_not_queued_until_the_device_is_linked() {
        tauri::async_runtime::block_on(async {
            let pool = test_pool().await;
            recipes::create(&pool, &recipe("offline")).await.unwrap();
            assert_eq!(queued(&pool).await, 0, "a local-only device keeps no outbox");

            sign_in(&pool, &SyncHandle::default(), SERVER, auth("Roelof")).await.unwrap();
            assert!(queued(&pool).await > 0, "signing in queues the local rows");
        });
    }

    #[test]
    fn status_reports_each_mode_and_hides_pending_unless_signed_in() {
        tauri::async_runtime::block_on(async {
            let pool = test_pool().await;
            let handle = SyncHandle::default();
            let id = recipes::create(&pool, &recipe("queued")).await.unwrap();

            // Linked but signed out: the queue has rows, and none of them count as pending.
            let mut conn = pool.acquire().await.unwrap();
            set_meta(&mut conn, USER_ID, &Uuid::new_v4().to_string()).await.unwrap();
            drop(conn);
            let mut conn = pool.acquire().await.unwrap();
            enqueue(&mut conn, EntityType::Recipe, &id, Operation::Upsert).await.unwrap();
            drop(conn);
            let signed_out = status(&pool, &handle).await.unwrap();
            assert_eq!(signed_out.mode, AccountMode::SignedOut);
            assert!(!signed_out.signed_in);
            assert_eq!(signed_out.pending, 0);
            assert!(signed_out.user.is_none());

            let mut conn = pool.acquire().await.unwrap();
            set_meta(&mut conn, TOKEN, "token-1").await.unwrap();
            drop(conn);
            let signed_in = status(&pool, &handle).await.unwrap();
            assert_eq!(signed_in.mode, AccountMode::SignedIn);
            assert!(signed_in.pending > 0);
        });
    }

    #[test]
    fn sign_in_stores_the_display_name_and_status_falls_back_to_the_email() {
        tauri::async_runtime::block_on(async {
            let pool = test_pool().await;
            let handle = SyncHandle::default();
            let named = status(&pool, &handle).await.unwrap();
            assert!(named.user.is_none());

            let first = auth("Roelof");
            let account = first.user.id;
            let status = sign_in(&pool, &handle, SERVER, first).await.unwrap();
            assert_eq!(status.user.as_ref().unwrap().name, "Roelof");
            assert_eq!(status.server_url.as_deref(), Some(SERVER));

            // The same account with no display name falls back to the part of the email before @.
            logout(&pool, &handle).await.unwrap();
            let mut unnamed = auth("");
            unnamed.user.id = account;
            let unnamed = sign_in(&pool, &handle, SERVER, unnamed).await.unwrap();
            assert_eq!(unnamed.user.as_ref().unwrap().name, "cook");
        });
    }

    #[test]
    fn signing_out_keeps_the_link_and_a_second_account_is_refused() {
        tauri::async_runtime::block_on(async {
            let pool = test_pool().await;
            let handle = SyncHandle::default();
            let first = auth("Roelof");
            let first_id = first.user.id;
            sign_in(&pool, &handle, SERVER, first).await.unwrap();

            let signed_out = logout(&pool, &handle).await.unwrap();
            assert_eq!(signed_out.mode, AccountMode::SignedOut);
            assert!(signed_out.user.is_none());
            assert_eq!(meta_value(&pool, USER_ID).await, Some(first_id.to_string()));
            assert_eq!(meta_value(&pool, DISPLAY_NAME).await, None);

            // The same account signs back in and keeps its link.
            let mut again = auth("Roelof");
            again.user.id = first_id;
            assert_eq!(sign_in(&pool, &handle, SERVER, again).await.unwrap().mode, AccountMode::SignedIn);

            logout(&pool, &handle).await.unwrap();
            let refused = sign_in(&pool, &handle, SERVER, auth("Someone else")).await;
            assert!(refused.is_err(), "a linked device cannot switch accounts");
        });
    }

    #[test]
    fn reset_to_local_moves_rows_off_the_family_and_clears_the_link() {
        tauri::async_runtime::block_on(async {
            let pool = test_pool().await;
            let handle = SyncHandle::default();
            sign_in(&pool, &handle, SERVER, auth("Roelof")).await.unwrap();
            let id = recipes::create(&pool, &recipe("shared")).await.unwrap();
            sqlx::query("UPDATE recipes SET family_id = 'family-1' WHERE id = ?")
                .bind(&id)
                .execute(&pool)
                .await
                .unwrap();
            let mut conn = pool.acquire().await.unwrap();
            set_meta(&mut conn, FAMILY_ID, "family-1").await.unwrap();
            set_meta(&mut conn, FAMILY_NAME, "Home").await.unwrap();
            set_meta(&mut conn, FAMILY_ROLE, "owner").await.unwrap();
            drop(conn);
            assert!(queued(&pool).await > 0);

            let status = reset_to_local(&pool, &handle).await.unwrap();
            assert_eq!(status.mode, AccountMode::Local);
            assert!(status.family.is_none());
            assert_eq!(status.server_url.as_deref(), Some(SERVER), "the server address is kept");
            assert_eq!(queued(&pool).await, 0);

            let owner: (String, Option<String>) = sqlx::query_as("SELECT owner_id, family_id FROM recipes WHERE id = ?")
                .bind(&id)
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(owner, ("local".to_string(), None));
            assert_eq!(meta_value(&pool, USER_ID).await, None);
            assert_eq!(meta_value(&pool, TOKEN).await, None);

            recipes::create(&pool, &recipe("after reset")).await.unwrap();
            assert_eq!(queued(&pool).await, 0, "a returned device stops queuing again");
        });
    }

    #[test]
    fn a_family_without_a_stored_role_counts_as_a_member() {
        tauri::async_runtime::block_on(async {
            let pool = test_pool().await;
            let mut conn = pool.acquire().await.unwrap();
            set_meta(&mut conn, FAMILY_ID, "family-1").await.unwrap();
            set_meta(&mut conn, FAMILY_NAME, "Home").await.unwrap();
            drop(conn);
            let family = status(&pool, &SyncHandle::default()).await.unwrap().family.unwrap();
            assert_eq!(family.role, FamilyRole::Member);
        });
    }

    async fn meta_value(pool: &SqlitePool, key: &str) -> Option<String> {
        sqlx::query_scalar("SELECT value FROM sync_meta WHERE key = ?")
            .bind(key)
            .fetch_optional(pool)
            .await
            .unwrap()
    }
}
