//! Local change log. Every repo write that changes a synced row calls `record` in the same
//! transaction, so a change is either saved and queued, or not saved at all.

use sqlx::SqliteConnection;

use shared_core::entity::{EntityType, Operation};

use crate::error::AppError;

/// Queues the current state of one row for push. Call it in the transaction that changed the row.
///
/// Soft deletes are recorded as `Delete`, but the payload is still the whole row (with
/// `deleted_at` set), so the server can apply it like any other change. A row that is already
/// pending is overwritten with its latest state, so repeated edits make one push.
pub async fn record(
    conn: &mut SqliteConnection,
    entity: EntityType,
    id: &str,
    operation: Operation,
) -> Result<(), AppError> {
    let (client_timestamp, payload): (String, String) = sqlx::query_as(snapshot_sql(entity))
        .bind(id)
        .fetch_one(&mut *conn)
        .await?;

    sqlx::query(
        "INSERT INTO outbox_queue (entity_type, entity_id, operation, payload, client_timestamp)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(entity_type, entity_id) WHERE status = 'pending' DO UPDATE SET
             operation = excluded.operation,
             payload = excluded.payload,
             client_timestamp = excluded.client_timestamp",
    )
    .bind(entity.as_str())
    .bind(id)
    .bind(operation.as_str())
    .bind(payload)
    .bind(client_timestamp)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Records every id in `ids` with the same operation.
pub async fn record_all(
    conn: &mut SqliteConnection,
    entity: EntityType,
    ids: &[String],
    operation: Operation,
) -> Result<(), AppError> {
    for id in ids {
        record(conn, entity, id, operation).await?;
    }
    Ok(())
}

/// The row as it goes over the wire, with `updated_at` first (it is the change's timestamp).
/// Device-only columns are left out: `image_path` and the `onboarded` flag. Stored JSON text is
/// embedded with `json()` so it is sent as JSON, not as a string.
fn snapshot_sql(entity: EntityType) -> &'static str {
    match entity {
        EntityType::Track => {
            "SELECT updated_at, json_object(
                'id', id, 'key', key, 'label', label, 'color', color, 'position', position,
                'default_time', default_time, 'visible', visible, 'repeats_on', repeats_on,
                'owner_id', owner_id, 'family_id', family_id,
                'updated_at', updated_at, 'deleted_at', deleted_at)
             FROM tracks WHERE id = ?"
        }
        EntityType::Tag => {
            "SELECT updated_at, json_object(
                'id', id, 'name', name,
                'owner_id', owner_id, 'family_id', family_id,
                'updated_at', updated_at, 'deleted_at', deleted_at)
             FROM tags WHERE id = ?"
        }
        EntityType::Recipe => {
            "SELECT updated_at, json_object(
                'id', id, 'name', name, 'url', url, 'source_domain', source_domain,
                'minutes', minutes, 'rating', rating, 'servings', servings, 'notes', notes,
                'usual_track_id', usual_track_id,
                'ingredients', json(ingredients), 'instructions', json(instructions),
                'image_url', image_url, 'created_at', created_at,
                'owner_id', owner_id, 'family_id', family_id,
                'updated_at', updated_at, 'deleted_at', deleted_at)
             FROM recipes WHERE id = ?"
        }
        EntityType::RecipeTag => {
            "SELECT updated_at, json_object(
                'id', id, 'recipe_id', recipe_id, 'tag_id', tag_id,
                'owner_id', owner_id, 'family_id', family_id,
                'updated_at', updated_at, 'deleted_at', deleted_at)
             FROM recipe_tags WHERE id = ?"
        }
        EntityType::ScheduleEntry => {
            "SELECT updated_at, json_object(
                'id', id, 'date', date, 'track_id', track_id, 'recipe_id', recipe_id,
                'owner_id', owner_id, 'family_id', family_id,
                'updated_at', updated_at, 'deleted_at', deleted_at)
             FROM schedule_entries WHERE id = ?"
        }
        EntityType::Settings => {
            "SELECT updated_at, json_object(
                'id', id, 'household_size', household_size, 'week_starts_on', week_starts_on,
                'owner_id', owner_id, 'family_id', family_id,
                'updated_at', updated_at, 'deleted_at', deleted_at)
             FROM settings WHERE id = ?"
        }
        EntityType::HouseholdDiet => {
            "SELECT updated_at, json_object(
                'id', id, 'diet', diet,
                'owner_id', owner_id, 'family_id', family_id,
                'updated_at', updated_at, 'deleted_at', deleted_at)
             FROM household_diets WHERE id = ?"
        }
        EntityType::Config => {
            "SELECT updated_at, json_object(
                'id', id, 'key', key, 'value', json(value),
                'owner_id', owner_id, 'family_id', family_id,
                'updated_at', updated_at, 'deleted_at', deleted_at)
             FROM config WHERE id = ?"
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::db::test_pool;
    use crate::models::{RecipeInput, SettingsInput, TrackInput};
    use crate::repo::{config, recipes, schedule, settings};
    use serde_json::{json, Value};
    use sqlx::SqlitePool;
    use tauri::async_runtime::block_on;

    fn recipe_input(name: &str, tags: &[&str]) -> RecipeInput {
        RecipeInput {
            name: name.to_string(),
            url: format!("https://example.com/{name}"),
            source_domain: "example.com".to_string(),
            minutes: Some(20),
            rating: None,
            servings: None,
            notes: None,
            usual_track_key: None,
            tags: tags.iter().map(|t| t.to_string()).collect(),
            ingredients: vec!["oats".to_string()],
            instructions: vec!["Stir.".to_string()],
            image_url: None,
        }
    }

    /// Queued changes as "entity OPERATION", in queue order.
    async fn queued(pool: &SqlitePool) -> Vec<String> {
        let rows: Vec<(String, String)> =
            sqlx::query_as("SELECT entity_type, operation FROM outbox_queue ORDER BY id")
                .fetch_all(pool)
                .await
                .unwrap();
        rows.into_iter().map(|(entity, op)| format!("{entity} {op}")).collect()
    }

    /// The payload of the only queued change for `entity_type`.
    async fn queued_payload(pool: &SqlitePool, entity_type: &str) -> Value {
        let payload: String = sqlx::query_scalar("SELECT payload FROM outbox_queue WHERE entity_type = ?")
            .bind(entity_type)
            .fetch_one(pool)
            .await
            .unwrap();
        serde_json::from_str(&payload).unwrap()
    }

    async fn clear_queue(pool: &SqlitePool) {
        sqlx::query("DELETE FROM outbox_queue").execute(pool).await.unwrap();
    }

    #[test]
    fn creating_a_recipe_queues_the_recipe_and_its_tag_links() {
        block_on(async {
            let pool = test_pool().await;
            recipes::create(&pool, &recipe_input("oats", &["Quick"])).await.unwrap();
            assert_eq!(queued(&pool).await, ["recipe UPSERT", "tag UPSERT", "recipe_tag UPSERT"]);
        });
    }

    #[test]
    fn deleting_a_recipe_queues_deletes_for_its_plan_and_tag_links() {
        block_on(async {
            let pool = test_pool().await;
            let id = recipes::create(&pool, &recipe_input("pie", &["Dinner"])).await.unwrap();
            schedule::set_entry(&pool, "2026-10-26", "dinner", &id).await.unwrap();
            clear_queue(&pool).await;

            recipes::delete(&pool, &id).await.unwrap();

            assert_eq!(
                queued(&pool).await,
                ["recipe DELETE", "schedule_entry DELETE", "recipe_tag DELETE"]
            );
            let payload = queued_payload(&pool, "recipe").await;
            assert!(payload["deleted_at"].is_string());
        });
    }

    #[test]
    fn repeated_edits_to_one_slot_coalesce_into_one_pending_change() {
        block_on(async {
            let pool = test_pool().await;
            let oats = recipes::create(&pool, &recipe_input("oats", &[])).await.unwrap();
            let toast = recipes::create(&pool, &recipe_input("toast", &[])).await.unwrap();
            clear_queue(&pool).await;

            schedule::set_entry(&pool, "2026-10-26", "breakfast", &oats).await.unwrap();
            schedule::set_entry(&pool, "2026-10-26", "breakfast", &toast).await.unwrap();

            assert_eq!(queued(&pool).await, ["schedule_entry UPSERT"]);
            let payload = queued_payload(&pool, "schedule_entry").await;
            assert_eq!(payload["recipe_id"], json!(toast));
        });
    }

    #[test]
    fn a_change_being_pushed_does_not_block_a_newer_one() {
        block_on(async {
            let pool = test_pool().await;
            let oats = recipes::create(&pool, &recipe_input("oats", &[])).await.unwrap();
            schedule::set_entry(&pool, "2026-10-26", "breakfast", &oats).await.unwrap();
            sqlx::query("UPDATE outbox_queue SET status = 'syncing'")
                .execute(&pool)
                .await
                .unwrap();

            schedule::set_entry(&pool, "2026-10-26", "breakfast", &oats).await.unwrap();

            let statuses: Vec<String> =
                sqlx::query_scalar("SELECT status FROM outbox_queue WHERE entity_type = 'schedule_entry' ORDER BY id")
                    .fetch_all(&pool)
                    .await
                    .unwrap();
            assert_eq!(statuses, ["syncing", "pending"]);
        });
    }

    #[test]
    fn removing_a_track_queues_its_meals_recipes_and_the_track() {
        block_on(async {
            let pool = test_pool().await;
            let mut input = recipe_input("oats", &[]);
            input.usual_track_key = Some("lunch".to_string());
            let oats = recipes::create(&pool, &input).await.unwrap();
            schedule::set_entry(&pool, "2026-10-26", "lunch", &oats).await.unwrap();
            clear_queue(&pool).await;

            let list: Vec<TrackInput> = schedule::tracks(&pool)
                .await
                .unwrap()
                .into_iter()
                .filter(|t| t.key != "lunch")
                .map(|t| TrackInput {
                    key: t.key,
                    label: t.label,
                    color: t.color,
                    default_time: t.default_time,
                    visible: t.visible,
                    repeats_on: t.repeats_on,
                })
                .collect();
            schedule::save_tracks(&pool, &list).await.unwrap();

            let queue = queued(&pool).await;
            let deletes: Vec<&String> = queue.iter().filter(|q| q.ends_with("DELETE")).collect();
            assert_eq!(deletes, ["schedule_entry DELETE", "track DELETE"]);
            assert!(queue.iter().any(|q| q == "recipe UPSERT"));
        });
    }

    #[test]
    fn saving_settings_queues_the_household_and_only_changed_diets() {
        block_on(async {
            let pool = test_pool().await;
            let input = |size: i64, dietary: &[&str]| SettingsInput {
                household_size: size,
                week_starts_on: "monday".to_string(),
                dietary: dietary.iter().map(|d| d.to_string()).collect(),
            };
            settings::save(&pool, &input(3, &["vegan", "nut-free"])).await.unwrap();
            clear_queue(&pool).await;

            settings::save(&pool, &input(3, &["vegetarian"])).await.unwrap();

            let queue = queued(&pool).await;
            let count = |change: &str| queue.iter().filter(|q| q.as_str() == change).count();
            assert_eq!(count("household_diet DELETE"), 2);
            assert_eq!(count("household_diet UPSERT"), 1);
            assert_eq!(count("settings UPSERT"), 1);
        });
    }

    #[test]
    fn settings_payload_leaves_out_the_device_onboarding_flag() {
        block_on(async {
            let pool = test_pool().await;
            settings::complete_onboarding(&pool).await.unwrap();
            settings::save(
                &pool,
                &SettingsInput {
                    household_size: 4,
                    week_starts_on: "sunday".to_string(),
                    dietary: vec![],
                },
            )
            .await
            .unwrap();

            let payload = queued_payload(&pool, "settings").await;
            assert!(payload.get("onboarded").is_none());
            assert_eq!(payload["household_size"], json!(4));
        });
    }

    #[test]
    fn config_payload_carries_the_value_as_json() {
        block_on(async {
            let pool = test_pool().await;
            config::set(&pool, "cook-keep-awake", &json!(true)).await.unwrap();

            let payload = queued_payload(&pool, "config").await;
            assert_eq!(payload["key"], json!("cook-keep-awake"));
            assert_eq!(payload["value"], json!(true));
        });
    }

    #[test]
    fn rows_created_after_joining_a_family_take_its_id() {
        block_on(async {
            let pool = test_pool().await;
            let before = recipes::create(&pool, &recipe_input("before", &[])).await.unwrap();
            sqlx::query("INSERT INTO sync_meta (key, value) VALUES ('active_family_id', 'family-1')")
                .execute(&pool)
                .await
                .unwrap();
            let after = recipes::create(&pool, &recipe_input("after", &[])).await.unwrap();

            let scopes: Vec<(String, Option<String>)> =
                sqlx::query_as("SELECT id, family_id FROM recipes ORDER BY name")
                    .fetch_all(&pool)
                    .await
                    .unwrap();
            assert_eq!(
                scopes,
                vec![
                    (after, Some("family-1".to_string())),
                    (before, None),
                ]
            );
        });
    }
}
