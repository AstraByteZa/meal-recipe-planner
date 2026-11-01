//! Family membership on this device. The server is the source of truth. Each session asks which
//! families the account is in and makes the device match. A removal or a deletion detaches the
//! family's rows, which then stay on the device as the person's private data.

use sqlx::SqlitePool;

use shared_core::entity::Operation;
use shared_core::wire::{FamilyDetails, InviteDto, InvitePreview};

use crate::error::AppError;
use crate::repo;
use crate::sync::account::{
    self, FAMILY_ID, FAMILY_INVITE, FAMILY_NAME, FAMILY_NOTICE, FAMILY_ROLE, FamilyInfo, FamilyNotice, SYNCED_TABLES,
    SyncStatus, active_family_info, role_text, store_active,
};
use crate::sync::client::{Api, FamilyDto, SyncError};
use crate::sync::outbox::enqueue;
use crate::sync::{SyncHandle, USER_ID, clear_meta, meta, set_meta};

#[derive(Debug, PartialEq)]
pub enum Reconcile {
    /// The device already matches the server.
    Keep,
    /// The active family is still listed, under a new name or role.
    Refresh(FamilyDto),
    /// The device has no active family and the account is in exactly one, so it becomes active.
    Activate(FamilyDto),
    /// The active family is no longer listed. The person was removed, or the family was deleted.
    Detach { family_id: String, name: String },
}

/// Compares the device's active family with the server's list. Pure, so the cases are tested.
pub fn decide(active: Option<&FamilyInfo>, listed: &[FamilyDto]) -> Reconcile {
    match active {
        Some(active) => match listed.iter().find(|family| family.id == active.id) {
            Some(family) if family.name == active.name && family.role == active.role => Reconcile::Keep,
            Some(family) => Reconcile::Refresh(family.clone()),
            None => Reconcile::Detach { family_id: active.id.clone(), name: active.name.clone() },
        },
        // With two or more families and none active, the choice is the user's, so nothing happens.
        None => match listed {
            [only] => Reconcile::Activate(only.clone()),
            _ => Reconcile::Keep,
        },
    }
}

/// Makes this device match the server's family list. Returns true when anything changed. Only a
/// successful list can detach a family, so an offline device keeps its family.
pub async fn reconcile(pool: &SqlitePool, api: &Api) -> Result<bool, SyncError> {
    let listed = api.families().await?;
    let active = {
        let mut conn = pool.acquire().await?;
        active_family_info(&mut conn).await?
    };
    match decide(active.as_ref(), &listed) {
        Reconcile::Keep => Ok(false),
        Reconcile::Refresh(family) => Ok(refresh(pool, &family).await?),
        Reconcile::Activate(family) => {
            let mut tx = pool.begin().await?;
            store_active(&mut *tx, &family, true).await?;
            tx.commit().await?;
            Ok(true)
        }
        Reconcile::Detach { family_id, name } => Ok(detach(pool, &family_id, Some(&name)).await?),
    }
}

/// Updates the name and role of the active family, if it is still the active one.
async fn refresh(pool: &SqlitePool, family: &FamilyDto) -> Result<bool, AppError> {
    let mut tx = pool.begin().await?;
    if meta(&mut *tx, FAMILY_ID).await?.as_deref() != Some(family.id.as_str()) {
        return Ok(false);
    }
    set_meta(&mut *tx, FAMILY_NAME, &family.name).await?;
    set_meta(&mut *tx, FAMILY_ROLE, role_text(family.role)).await?;
    tx.commit().await?;
    Ok(true)
}

/// Removes `family_id` from this device. Its rows stay as the signed-in person's private data and
/// are queued, so the server receives the private copy. A removal passes a `notice` name, which
/// is kept for the user to read. A voluntary leave or delete passes `None`, which also clears any
/// old notice. Returns false when the device was no longer in that family, so a stale decision
/// does nothing.
pub async fn detach(pool: &SqlitePool, family_id: &str, notice: Option<&str>) -> Result<bool, AppError> {
    let mut tx = pool.begin().await?;
    let detached = meta(&mut *tx, FAMILY_ID).await?.as_deref() == Some(family_id);
    if detached {
        let me = meta(&mut *tx, USER_ID)
            .await?
            .ok_or_else(|| AppError::Invalid("This device is not signed in.".to_string()))?;
        let now = repo::now(&mut *tx).await?;
        for (table, entity) in SYNCED_TABLES {
            let rows: Vec<(String, Option<String>)> =
                sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT id, deleted_at FROM {table} WHERE family_id = ?")))
                    .bind(family_id)
                    .fetch_all(&mut *tx)
                    .await?;
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "UPDATE {table} SET family_id = NULL, owner_id = ?, updated_at = ? WHERE family_id = ?"
            )))
            .bind(me.as_str())
            .bind(now.as_str())
            .bind(family_id)
            .execute(&mut *tx)
            .await?;
            for (id, deleted_at) in rows {
                let operation = if deleted_at.is_some() { Operation::Delete } else { Operation::Upsert };
                enqueue(&mut *tx, entity, &id, operation).await?;
            }
        }
        for key in [FAMILY_ID, FAMILY_NAME, FAMILY_ROLE, FAMILY_INVITE] {
            clear_meta(&mut *tx, key).await?;
        }
    }
    match notice {
        Some(family_name) if detached => {
            let notice = FamilyNotice { kind: "removed".to_string(), family_name: family_name.to_string() };
            let json = serde_json::to_string(&notice).map_err(|error| AppError::Invalid(error.to_string()))?;
            set_meta(&mut *tx, FAMILY_NOTICE, &json).await?;
        }
        Some(_) => {}
        None => clear_meta(&mut *tx, FAMILY_NOTICE).await?,
    }
    tx.commit().await?;
    Ok(detached)
}

/// The active family and a client for it. Fails when the device is not in one.
async fn active(pool: &SqlitePool) -> Result<(Api, FamilyInfo), AppError> {
    let session = account::require_session(pool).await?;
    let family = {
        let mut conn = pool.acquire().await?;
        active_family_info(&mut conn).await?
    }
    .ok_or_else(|| AppError::Invalid("Join or create a family first.".to_string()))?;
    Ok((Api::new(&session.server_url, Some(&session.token))?, family))
}

/// Runs after a command has changed the server. A leave or delete has already detached the family
/// here. This re-reads the family, and restarts the worker so its stream follows the change.
async fn settle(pool: &SqlitePool, handle: &SyncHandle, api: &Api, detached: bool) -> Result<SyncStatus, AppError> {
    let changed = reconcile(pool, api).await?;
    if detached || changed {
        handle.restart();
    }
    account::status(pool, handle).await
}

pub async fn details(pool: &SqlitePool) -> Result<FamilyDetails, AppError> {
    let (api, family) = active(pool).await?;
    Ok(api.family_details(&family.id).await?)
}

pub async fn invite_create(pool: &SqlitePool) -> Result<InviteDto, AppError> {
    let (api, family) = active(pool).await?;
    Ok(api.create_invite(&family.id).await?)
}

pub async fn invite_revoke(pool: &SqlitePool, invite_id: &str) -> Result<(), AppError> {
    let (api, family) = active(pool).await?;
    Ok(api.revoke_invite(&family.id, invite_id).await?)
}

/// Looks up a code without using it. Works before the device is in a family.
pub async fn invite_preview(pool: &SqlitePool, code: &str) -> Result<InvitePreview, AppError> {
    let session = account::require_session(pool).await?;
    let api = Api::new(&session.server_url, Some(&session.token))?;
    Ok(api.preview_invite(code.trim()).await?)
}

pub async fn rename(pool: &SqlitePool, handle: &SyncHandle, name: &str) -> Result<SyncStatus, AppError> {
    let (api, family) = active(pool).await?;
    api.rename_family(&family.id, name.trim()).await?;
    settle(pool, handle, &api, false).await
}

/// An owner removes a member. A member who passes their own id leaves the family.
pub async fn member_remove(pool: &SqlitePool, handle: &SyncHandle, user_id: &str) -> Result<SyncStatus, AppError> {
    let (api, family) = active(pool).await?;
    api.remove_member(&family.id, user_id).await?;
    let me = {
        let mut conn = pool.acquire().await?;
        meta(&mut conn, USER_ID).await?
    };
    let left = me.as_deref() == Some(user_id);
    if left {
        detach(pool, &family.id, None).await?;
    }
    settle(pool, handle, &api, left).await
}

pub async fn transfer(pool: &SqlitePool, handle: &SyncHandle, user_id: &str) -> Result<SyncStatus, AppError> {
    let (api, family) = active(pool).await?;
    api.transfer_ownership(&family.id, user_id).await?;
    settle(pool, handle, &api, false).await
}

/// The owner deletes the family for everyone. This device keeps its rows as private data.
pub async fn delete(pool: &SqlitePool, handle: &SyncHandle) -> Result<SyncStatus, AppError> {
    let (api, family) = active(pool).await?;
    api.delete_family(&family.id).await?;
    detach(pool, &family.id, None).await?;
    settle(pool, handle, &api, true).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_pool;
    use crate::models::RecipeInput;
    use crate::repo::recipes;
    use crate::sync::SyncHandle;
    use crate::sync::account::{AccountMode, sign_in};
    use crate::sync::client::AuthResponse;
    use shared_core::wire::{FamilyRole, UserDto};
    use uuid::Uuid;

    fn info(id: &str, name: &str, role: FamilyRole) -> FamilyInfo {
        FamilyInfo { id: id.to_string(), name: name.to_string(), role }
    }

    fn listed(id: &str, name: &str, role: FamilyRole) -> FamilyDto {
        FamilyDto { id: id.to_string(), name: name.to_string(), role }
    }

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
    fn reconcile_decisions() {
        let home = info("f1", "Home", FamilyRole::Member);
        let other = listed("f2", "Other", FamilyRole::Owner);
        let still_home = listed("f1", "Home", FamilyRole::Member);

        // No active family: join the only family listed, but never pick between several.
        assert_eq!(decide(None, &[]), Reconcile::Keep);
        assert_eq!(decide(None, std::slice::from_ref(&other)), Reconcile::Activate(other.clone()));
        assert_eq!(decide(None, &[other.clone(), still_home.clone()]), Reconcile::Keep);

        // The active family is listed: keep it, or refresh its name or role when they changed.
        assert_eq!(decide(Some(&home), std::slice::from_ref(&still_home)), Reconcile::Keep);
        let renamed = listed("f1", "Household", FamilyRole::Member);
        assert_eq!(decide(Some(&home), std::slice::from_ref(&renamed)), Reconcile::Refresh(renamed));
        let promoted = listed("f1", "Home", FamilyRole::Owner);
        assert_eq!(decide(Some(&home), std::slice::from_ref(&promoted)), Reconcile::Refresh(promoted));

        // The active family is gone. Another family in the list does not take its place.
        let gone = Reconcile::Detach { family_id: "f1".to_string(), name: "Home".to_string() };
        assert_eq!(decide(Some(&home), &[]), gone);
        assert_eq!(decide(Some(&home), std::slice::from_ref(&other)), gone);
    }

    async fn signed_in(pool: &SqlitePool) -> Uuid {
        let handle = SyncHandle::default();
        let id = Uuid::new_v4();
        let auth = AuthResponse {
            token: "token-1".to_string(),
            user: UserDto { id, email: "cook@example.com".to_string(), display_name: "Cook".to_string() },
        };
        sign_in(pool, &handle, "https://sync.example.com", auth).await.unwrap();
        id
    }

    async fn join_family_locally(pool: &SqlitePool, family_id: &str, recipe_id: &str) {
        sqlx::query("UPDATE recipes SET family_id = ? WHERE id = ?")
            .bind(family_id)
            .bind(recipe_id)
            .execute(pool)
            .await
            .unwrap();
        let mut conn = pool.acquire().await.unwrap();
        set_meta(&mut conn, FAMILY_ID, family_id).await.unwrap();
        set_meta(&mut conn, FAMILY_NAME, "Home").await.unwrap();
        set_meta(&mut conn, FAMILY_ROLE, "member").await.unwrap();
        drop(conn);
        sqlx::query("DELETE FROM outbox_queue").execute(pool).await.unwrap();
    }

    #[test]
    fn a_removed_family_becomes_private_data_with_a_notice() {
        tauri::async_runtime::block_on(async {
            let pool = test_pool().await;
            let me = signed_in(&pool).await;
            let id = recipes::create(&pool, &recipe("Pancakes")).await.unwrap();
            join_family_locally(&pool, "f1", &id).await;

            assert!(detach(&pool, "f1", Some("Home")).await.unwrap());

            let row: (Option<String>, String) = sqlx::query_as("SELECT family_id, owner_id FROM recipes WHERE id = ?")
                .bind(&id)
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(row, (None, me.to_string()), "the row is private and owned by this person");

            let payload: String = sqlx::query_scalar("SELECT payload FROM outbox_queue WHERE entity_id = ?")
                .bind(&id)
                .fetch_one(&pool)
                .await
                .unwrap();
            assert!(payload.contains("\"family_id\":null"), "the queued copy no longer names the family");

            let status = account::status(&pool, &SyncHandle::default()).await.unwrap();
            assert_eq!(status.mode, AccountMode::SignedIn, "the account stays signed in");
            assert!(status.family.is_none());
            assert_eq!(status.notice.expect("a notice").family_name, "Home");
        });
    }

    #[test]
    fn a_detach_that_is_already_done_changes_nothing_and_a_leave_clears_the_notice() {
        tauri::async_runtime::block_on(async {
            let pool = test_pool().await;
            signed_in(&pool).await;
            let id = recipes::create(&pool, &recipe("Soup")).await.unwrap();
            join_family_locally(&pool, "f1", &id).await;
            assert!(detach(&pool, "f1", Some("Home")).await.unwrap());

            // A second decision based on the old view does nothing.
            assert!(!detach(&pool, "f1", Some("Home")).await.unwrap());

            // A voluntary leave by another route clears the notice.
            assert!(!detach(&pool, "f1", None).await.unwrap());
            let status = account::status(&pool, &SyncHandle::default()).await.unwrap();
            assert!(status.notice.is_none());
        });
    }
}
