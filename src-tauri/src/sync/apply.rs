//! Writes ops pulled from the server into the local tables. Applied changes are not queued again,
//! so a pulled change never echoes back to the server.
//!
//! Each op runs in its own savepoint. An op that breaks a constraint on this device (say, a
//! schedule entry for a track this device cannot see) is logged and skipped, so one row cannot
//! block the rest of the page. Other errors stop the page, and the caller retries from the last
//! saved cursor.

use sqlx::SqliteConnection;

use shared_core::entity::{EntityType, Operation};
use shared_core::wire::PullOp;

use crate::error::AppError;
use crate::sync::outbox;

/// Applies one page of pulled ops. Returns true if any local row changed.
pub async fn apply_page(conn: &mut SqliteConnection, local_client: &str, ops: &[PullOp]) -> Result<bool, AppError> {
    let mut changed = false;
    for op in ops {
        sqlx::query("SAVEPOINT pulled_op").execute(&mut *conn).await?;
        match apply_op(&mut *conn, local_client, op).await {
            Ok(applied) => {
                changed |= applied;
                sqlx::query("RELEASE pulled_op").execute(&mut *conn).await?;
            }
            Err(error) => {
                sqlx::query("ROLLBACK TO pulled_op").execute(&mut *conn).await?;
                sqlx::query("RELEASE pulled_op").execute(&mut *conn).await?;
                if !is_bad_row(&error) {
                    return Err(error);
                }
                eprintln!("sync: skipped {} {} op {}: {error}", op.entity_type.as_str(), op.entity_id, op.seq);
            }
        }
    }
    Ok(changed)
}

/// Constraint failures come from the row itself, so retrying will not help. Other errors (a
/// locked or broken database) are not the row's fault.
fn is_bad_row(error: &AppError) -> bool {
    use sqlx::error::ErrorKind;
    match error {
        AppError::Db(sqlx::Error::Database(db)) => matches!(
            db.kind(),
            ErrorKind::UniqueViolation | ErrorKind::ForeignKeyViolation | ErrorKind::NotNullViolation | ErrorKind::CheckViolation
        ),
        _ => false,
    }
}

async fn apply_op(conn: &mut SqliteConnection, local_client: &str, op: &PullOp) -> Result<bool, AppError> {
    // This device's own changes are already applied locally.
    if op.client_id == local_client {
        return Ok(false);
    }
    let id = op.entity_id.to_string();
    let incoming_ts = op.client_timestamp.as_str();

    // A row already at least this new stays. An equal timestamp is a replay of an op already applied.
    let local_ts = local_updated_at(conn, op.entity_type, &id).await?;
    if local_ts.as_deref().is_some_and(|ts| ts >= incoming_ts) {
        return Ok(false);
    }

    // Local edits not yet pushed can be newer than this op. If so, they win and the op is skipped.
    if !outbox::claim_entity(conn, op.entity_type, &id, incoming_ts, &op.client_id, local_client).await? {
        return Ok(false);
    }

    match op.op {
        Operation::Upsert => {
            sqlx::query(upsert_sql(op.entity_type))
                .bind(op.payload.to_string())
                .execute(&mut *conn)
                .await?;
        }
        // A delete is a soft delete stamped with the op's time. The row's other fields are left as
        // they were, since nothing shows a deleted row, and a later upsert overwrites them all.
        Operation::Delete => {
            let sql = format!("UPDATE {} SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2", table_name(op.entity_type));
            sqlx::query(sqlx::AssertSqlSafe(sql))
                .bind(incoming_ts)
                .bind(&id)
                .execute(&mut *conn)
                .await?;
        }
    }
    Ok(true)
}

async fn local_updated_at(conn: &mut SqliteConnection, entity: EntityType, id: &str) -> Result<Option<String>, AppError> {
    // The table name comes from `table_name`, which only returns fixed names.
    let sql = format!("SELECT updated_at FROM {} WHERE id = ?", table_name(entity));
    let ts = sqlx::query_scalar(sqlx::AssertSqlSafe(sql)).bind(id).fetch_optional(conn).await?;
    Ok(ts)
}

fn table_name(entity: EntityType) -> &'static str {
    match entity {
        EntityType::Track => "tracks",
        EntityType::Tag => "tags",
        EntityType::Recipe => "recipes",
        EntityType::RecipeTag => "recipe_tags",
        EntityType::ScheduleEntry => "schedule_entries",
        EntityType::Settings => "settings",
        EntityType::HouseholdDiet => "household_diets",
        EntityType::Config => "config",
    }
}

/// Inserts or replaces a row from its payload (`?1`). Device-only columns are not in the payload
/// and are left alone: `image_path`, and the settings `onboarded` flag, which a pulled settings row
/// sets, so a new device skips onboarding.
fn upsert_sql(entity: EntityType) -> &'static str {
    match entity {
        EntityType::Track => {
            "INSERT INTO tracks (id, key, label, color, position, default_time, visible, repeats_on,
                                 owner_id, family_id, updated_at, deleted_at)
             VALUES (json_extract(?1, '$.id'), json_extract(?1, '$.key'), json_extract(?1, '$.label'),
                     json_extract(?1, '$.color'), json_extract(?1, '$.position'), json_extract(?1, '$.default_time'),
                     json_extract(?1, '$.visible'), json_extract(?1, '$.repeats_on'), json_extract(?1, '$.owner_id'),
                     json_extract(?1, '$.family_id'), json_extract(?1, '$.updated_at'), json_extract(?1, '$.deleted_at'))
             ON CONFLICT(id) DO UPDATE SET
                 key = excluded.key, label = excluded.label, color = excluded.color, position = excluded.position,
                 default_time = excluded.default_time, visible = excluded.visible, repeats_on = excluded.repeats_on,
                 owner_id = excluded.owner_id, family_id = excluded.family_id,
                 updated_at = excluded.updated_at, deleted_at = excluded.deleted_at"
        }
        EntityType::Tag => {
            "INSERT INTO tags (id, name, owner_id, family_id, updated_at, deleted_at)
             VALUES (json_extract(?1, '$.id'), json_extract(?1, '$.name'), json_extract(?1, '$.owner_id'),
                     json_extract(?1, '$.family_id'), json_extract(?1, '$.updated_at'), json_extract(?1, '$.deleted_at'))
             ON CONFLICT(id) DO UPDATE SET
                 name = excluded.name, owner_id = excluded.owner_id, family_id = excluded.family_id,
                 updated_at = excluded.updated_at, deleted_at = excluded.deleted_at"
        }
        // The usual track is only linked when this device has that track. A track private to
        // another account is not visible here, and the recipe should still arrive.
        EntityType::Recipe => {
            "INSERT INTO recipes (id, name, url, source_domain, minutes, rating, servings, notes, usual_track_id,
                                  ingredients, instructions, image_url, created_at, owner_id, family_id,
                                  updated_at, deleted_at)
             VALUES (json_extract(?1, '$.id'), json_extract(?1, '$.name'), json_extract(?1, '$.url'),
                     json_extract(?1, '$.source_domain'), json_extract(?1, '$.minutes'), json_extract(?1, '$.rating'),
                     json_extract(?1, '$.servings'), json_extract(?1, '$.notes'),
                     (SELECT id FROM tracks WHERE id = json_extract(?1, '$.usual_track_id')),
                     ?1 -> '$.ingredients', ?1 -> '$.instructions', json_extract(?1, '$.image_url'),
                     json_extract(?1, '$.created_at'), json_extract(?1, '$.owner_id'), json_extract(?1, '$.family_id'),
                     json_extract(?1, '$.updated_at'), json_extract(?1, '$.deleted_at'))
             ON CONFLICT(id) DO UPDATE SET
                 name = excluded.name, url = excluded.url, source_domain = excluded.source_domain,
                 minutes = excluded.minutes, rating = excluded.rating, servings = excluded.servings,
                 notes = excluded.notes, usual_track_id = excluded.usual_track_id,
                 ingredients = excluded.ingredients, instructions = excluded.instructions,
                 image_url = excluded.image_url, created_at = excluded.created_at,
                 owner_id = excluded.owner_id, family_id = excluded.family_id,
                 updated_at = excluded.updated_at, deleted_at = excluded.deleted_at"
        }
        EntityType::RecipeTag => {
            "INSERT INTO recipe_tags (id, recipe_id, tag_id, owner_id, family_id, updated_at, deleted_at)
             VALUES (json_extract(?1, '$.id'), json_extract(?1, '$.recipe_id'), json_extract(?1, '$.tag_id'),
                     json_extract(?1, '$.owner_id'), json_extract(?1, '$.family_id'),
                     json_extract(?1, '$.updated_at'), json_extract(?1, '$.deleted_at'))
             ON CONFLICT(id) DO UPDATE SET
                 recipe_id = excluded.recipe_id, tag_id = excluded.tag_id, owner_id = excluded.owner_id,
                 family_id = excluded.family_id, updated_at = excluded.updated_at, deleted_at = excluded.deleted_at"
        }
        EntityType::ScheduleEntry => {
            "INSERT INTO schedule_entries (id, date, track_id, recipe_id, owner_id, family_id, updated_at, deleted_at)
             VALUES (json_extract(?1, '$.id'), json_extract(?1, '$.date'), json_extract(?1, '$.track_id'),
                     json_extract(?1, '$.recipe_id'), json_extract(?1, '$.owner_id'), json_extract(?1, '$.family_id'),
                     json_extract(?1, '$.updated_at'), json_extract(?1, '$.deleted_at'))
             ON CONFLICT(id) DO UPDATE SET
                 date = excluded.date, track_id = excluded.track_id, recipe_id = excluded.recipe_id,
                 owner_id = excluded.owner_id, family_id = excluded.family_id,
                 updated_at = excluded.updated_at, deleted_at = excluded.deleted_at"
        }
        EntityType::Settings => {
            "INSERT INTO settings (id, household_size, week_starts_on, onboarded, owner_id, family_id, updated_at, deleted_at)
             VALUES (json_extract(?1, '$.id'), json_extract(?1, '$.household_size'), json_extract(?1, '$.week_starts_on'),
                     1, json_extract(?1, '$.owner_id'), json_extract(?1, '$.family_id'),
                     json_extract(?1, '$.updated_at'), json_extract(?1, '$.deleted_at'))
             ON CONFLICT(id) DO UPDATE SET
                 household_size = excluded.household_size, week_starts_on = excluded.week_starts_on,
                 onboarded = 1, owner_id = excluded.owner_id, family_id = excluded.family_id,
                 updated_at = excluded.updated_at, deleted_at = excluded.deleted_at"
        }
        EntityType::HouseholdDiet => {
            "INSERT INTO household_diets (id, diet, owner_id, family_id, updated_at, deleted_at)
             VALUES (json_extract(?1, '$.id'), json_extract(?1, '$.diet'), json_extract(?1, '$.owner_id'),
                     json_extract(?1, '$.family_id'), json_extract(?1, '$.updated_at'), json_extract(?1, '$.deleted_at'))
             ON CONFLICT(id) DO UPDATE SET
                 diet = excluded.diet, owner_id = excluded.owner_id, family_id = excluded.family_id,
                 updated_at = excluded.updated_at, deleted_at = excluded.deleted_at"
        }
        EntityType::Config => {
            "INSERT INTO config (id, key, value, owner_id, family_id, updated_at, deleted_at)
             VALUES (json_extract(?1, '$.id'), json_extract(?1, '$.key'), ?1 -> '$.value', json_extract(?1, '$.owner_id'),
                     json_extract(?1, '$.family_id'), json_extract(?1, '$.updated_at'), json_extract(?1, '$.deleted_at'))
             ON CONFLICT(id) DO UPDATE SET
                 key = excluded.key, value = excluded.value, owner_id = excluded.owner_id,
                 family_id = excluded.family_id, updated_at = excluded.updated_at, deleted_at = excluded.deleted_at"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_pool;
    use crate::models::RecipeInput;
    use crate::repo::{recipes, schedule, settings};
    use serde_json::{Value, json};
    use sqlx::SqlitePool;
    use tauri::async_runtime::block_on;
    use uuid::Uuid;

    const LOCAL: &str = "this-device";
    const OTHER: &str = "other-device";
    const RECIPE: &str = "11111111-1111-4111-8111-111111111111";
    const EARLY: &str = "2026-10-01T10:00:00.000Z";
    const LATE: &str = "2026-10-02T10:00:00.000Z";

    fn remote(seq: i64, client: &str, entity: EntityType, id: &str, op: Operation, ts: &str, payload: Value) -> PullOp {
        PullOp {
            seq,
            entity_type: entity,
            entity_id: Uuid::parse_str(id).unwrap(),
            op,
            family_id: None,
            client_id: client.to_string(),
            client_timestamp: ts.to_string(),
            payload,
        }
    }

    fn recipe_payload(name: &str, ts: &str) -> Value {
        json!({
            "id": RECIPE, "name": name, "url": "", "source_domain": "", "minutes": 20, "rating": null,
            "servings": null, "notes": null, "usual_track_id": null, "ingredients": ["oats"],
            "instructions": ["Stir."], "image_url": null, "created_at": EARLY,
            "owner_id": "user-b", "family_id": null, "updated_at": ts, "deleted_at": null,
        })
    }

    async fn recipe_name(pool: &SqlitePool) -> String {
        sqlx::query_scalar("SELECT name FROM recipes WHERE id = ?")
            .bind(RECIPE)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn apply(pool: &SqlitePool, ops: &[PullOp]) -> bool {
        let mut conn = pool.acquire().await.unwrap();
        apply_page(&mut conn, LOCAL, ops).await.unwrap()
    }

    #[test]
    fn a_new_remote_recipe_is_inserted_with_its_fields() {
        block_on(async {
            let pool = test_pool().await;
            let op = remote(1, OTHER, EntityType::Recipe, RECIPE, Operation::Upsert, EARLY, recipe_payload("Soup", EARLY));
            assert!(apply(&pool, &[op]).await);

            let (ingredients, owner): (String, String) =
                sqlx::query_as("SELECT ingredients, owner_id FROM recipes WHERE id = ?")
                    .bind(RECIPE)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(ingredients, r#"["oats"]"#);
            assert_eq!(owner, "user-b");
            assert_eq!(recipe_name(&pool).await, "Soup");
        });
    }

    #[test]
    fn ops_from_this_device_are_skipped() {
        block_on(async {
            let pool = test_pool().await;
            let op = remote(1, LOCAL, EntityType::Recipe, RECIPE, Operation::Upsert, EARLY, recipe_payload("Soup", EARLY));
            assert!(!apply(&pool, &[op]).await);
            assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM recipes").fetch_one(&pool).await.unwrap(), 0);
        });
    }

    #[test]
    fn a_newer_remote_version_replaces_an_older_local_row_and_an_older_one_is_ignored() {
        block_on(async {
            let pool = test_pool().await;
            apply(&pool, &[remote(1, OTHER, EntityType::Recipe, RECIPE, Operation::Upsert, EARLY, recipe_payload("Soup", EARLY))]).await;

            apply(&pool, &[remote(2, OTHER, EntityType::Recipe, RECIPE, Operation::Upsert, LATE, recipe_payload("Stew", LATE))]).await;
            assert_eq!(recipe_name(&pool).await, "Stew");

            let older = remote(3, OTHER, EntityType::Recipe, RECIPE, Operation::Upsert, EARLY, recipe_payload("Old", EARLY));
            assert!(!apply(&pool, &[older.clone(), older]).await);
            assert_eq!(recipe_name(&pool).await, "Stew");
        });
    }

    #[test]
    fn a_newer_local_row_is_not_overwritten_by_an_older_remote_op() {
        block_on(async {
            let pool = test_pool().await;
            let local = recipes::create(
                &pool,
                &RecipeInput {
                    name: "Mine".to_string(),
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
                },
            )
            .await
            .unwrap();
            // Pulled ops from the server are old compared with a row just created here.
            let old = remote(1, OTHER, EntityType::Recipe, &local, Operation::Upsert, EARLY, {
                let mut payload = recipe_payload("Theirs", EARLY);
                payload["id"] = json!(local);
                payload
            });
            assert!(!apply(&pool, &[old]).await);
            let name: String = sqlx::query_scalar("SELECT name FROM recipes WHERE id = ?")
                .bind(&local)
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(name, "Mine");
        });
    }

    #[test]
    fn a_pending_local_edit_newer_than_the_remote_op_survives_and_an_older_one_is_superseded() {
        block_on(async {
            let pool = test_pool().await;
            apply(&pool, &[remote(1, OTHER, EntityType::Recipe, RECIPE, Operation::Upsert, EARLY, recipe_payload("Soup", EARLY))]).await;
            // A local edit that is still queued, stamped after the remote one.
            sqlx::query("INSERT INTO outbox_queue (entity_type, entity_id, operation, payload, client_timestamp) VALUES ('recipe', ?, 'UPSERT', '{}', ?)")
                .bind(RECIPE)
                .bind(LATE)
                .execute(&pool)
                .await
                .unwrap();
            // The pulled op is between the two. The local queued edit is newer than it, so it wins.
            let middle = "2026-10-01T23:00:00.000Z";
            assert!(!apply(&pool, &[remote(2, OTHER, EntityType::Recipe, RECIPE, Operation::Upsert, middle, recipe_payload("Middle", middle))]).await);
            assert_eq!(recipe_name(&pool).await, "Soup");
            assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM outbox_queue").fetch_one(&pool).await.unwrap(), 1);

            // A pulled op newer than the queued edit wins, and the superseded queue row goes.
            let newer = "2026-10-03T10:00:00.000Z";
            assert!(apply(&pool, &[remote(3, OTHER, EntityType::Recipe, RECIPE, Operation::Upsert, newer, recipe_payload("Newest", newer))]).await);
            assert_eq!(recipe_name(&pool).await, "Newest");
            assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM outbox_queue").fetch_one(&pool).await.unwrap(), 0);
        });
    }

    #[test]
    fn a_delete_soft_deletes_the_row_with_the_op_time() {
        block_on(async {
            let pool = test_pool().await;
            apply(&pool, &[remote(1, OTHER, EntityType::Recipe, RECIPE, Operation::Upsert, EARLY, recipe_payload("Soup", EARLY))]).await;
            apply(&pool, &[remote(2, OTHER, EntityType::Recipe, RECIPE, Operation::Delete, LATE, Value::Null)]).await;

            let (deleted, updated): (Option<String>, String) =
                sqlx::query_as("SELECT deleted_at, updated_at FROM recipes WHERE id = ?")
                    .bind(RECIPE)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(deleted.as_deref(), Some(LATE));
            assert_eq!(updated, LATE);
        });
    }

    #[test]
    fn a_pulled_settings_row_marks_this_device_onboarded() {
        block_on(async {
            let pool = test_pool().await;
            let id = settings::SETTINGS_ID;
            let payload = json!({
                "id": id, "household_size": 5, "week_starts_on": "sunday",
                "owner_id": "user-b", "family_id": null, "updated_at": LATE, "deleted_at": null,
            });
            apply(&pool, &[remote(1, OTHER, EntityType::Settings, id, Operation::Upsert, LATE, payload)]).await;

            let (size, onboarded): (i64, i64) = sqlx::query_as("SELECT household_size, onboarded FROM settings WHERE id = ?")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!((size, onboarded), (5, 1));
        });
    }

    #[test]
    fn a_seeded_track_loses_to_any_real_change_from_another_device() {
        block_on(async {
            let pool = test_pool().await;
            let breakfast = "00000000-0000-4000-8000-000000000001";
            let payload = json!({
                "id": breakfast, "key": "breakfast", "label": "Morning", "color": "orange", "position": 0,
                "default_time": "08:00", "visible": 1, "repeats_on": "1111111", "owner_id": "user-b",
                "family_id": null, "updated_at": "2026-10-01T00:00:00.000Z", "deleted_at": null,
            });
            apply(&pool, &[remote(1, OTHER, EntityType::Track, breakfast, Operation::Upsert, "2026-10-01T00:00:00.000Z", payload)]).await;
            let label: String = sqlx::query_scalar("SELECT label FROM tracks WHERE id = ?")
                .bind(breakfast)
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(label, "Morning");
        });
    }

    #[test]
    fn an_op_that_breaks_a_constraint_is_skipped_and_the_page_continues() {
        block_on(async {
            let pool = test_pool().await;
            // The schedule entry points at a track and recipe this device does not have.
            let missing = remote(
                1,
                OTHER,
                EntityType::ScheduleEntry,
                "22222222-2222-4222-8222-222222222222",
                Operation::Upsert,
                EARLY,
                json!({
                    "id": "22222222-2222-4222-8222-222222222222", "date": "2026-10-26",
                    "track_id": "33333333-3333-4333-8333-333333333333", "recipe_id": RECIPE,
                    "owner_id": "user-b", "family_id": null, "updated_at": EARLY, "deleted_at": null,
                }),
            );
            let good = remote(2, OTHER, EntityType::Recipe, RECIPE, Operation::Upsert, EARLY, recipe_payload("Soup", EARLY));
            assert!(apply(&pool, &[missing, good]).await);
            assert_eq!(recipe_name(&pool).await, "Soup");
            assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM schedule_entries").fetch_one(&pool).await.unwrap(), 0);
        });
    }

    #[test]
    fn a_pulled_slot_links_to_the_local_track_it_names() {
        block_on(async {
            let pool = test_pool().await;
            let oats = recipes::create(
                &pool,
                &RecipeInput {
                    name: "Oats".to_string(),
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
                },
            )
            .await
            .unwrap();
            let track = "00000000-0000-4000-8000-000000000004";
            let slot = shared_core::ids::slot_id("2026-10-26", track);
            let payload = json!({
                "id": slot, "date": "2026-10-26", "track_id": track, "recipe_id": oats,
                "owner_id": "user-b", "family_id": null, "updated_at": LATE, "deleted_at": null,
            });
            apply(&pool, &[remote(1, OTHER, EntityType::ScheduleEntry, &slot, Operation::Upsert, LATE, payload)]).await;

            let planned = schedule::week(&pool, "2026-10-26", "2026-10-26").await.unwrap();
            assert_eq!(planned.len(), 1);
            assert_eq!(planned[0].track_key, "dinner");
        });
    }

}
