use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{PlannedMeal, Track, TrackInput};
use crate::repo::{now, slot_id};
use crate::sync::active_family;
use shared_core::entity::{EntityType, Operation};
use crate::sync::outbox::{record, record_all};

pub async fn tracks(pool: &SqlitePool) -> Result<Vec<Track>, AppError> {
    let tracks = sqlx::query_as(
        "SELECT key, label, color, position, default_time, visible, repeats_on
         FROM tracks WHERE deleted_at IS NULL ORDER BY position",
    )
    .fetch_all(pool)
    .await?;
    Ok(tracks)
}

/// Replaces the whole track list with `tracks`, in the order given.
///
/// Tracks missing from the list are soft-deleted. Their planned meals go with them, and recipes
/// that used them as their usual track forget it.
pub async fn save_tracks(pool: &SqlitePool, tracks: &[TrackInput]) -> Result<(), AppError> {
    validate_tracks(tracks)?;

    let mut tx = pool.begin().await?;
    let now = now(&mut *tx).await?;

    let live: Vec<String> = sqlx::query_scalar("SELECT key FROM tracks WHERE deleted_at IS NULL")
        .fetch_all(&mut *tx)
        .await?;
    for key in live.iter().filter(|key| !tracks.iter().any(|t| &t.key == *key)) {
        let entry_ids: Vec<String> = sqlx::query_scalar(
            "UPDATE schedule_entries SET deleted_at = ?, updated_at = ?
             WHERE deleted_at IS NULL AND track_id = (SELECT id FROM tracks WHERE key = ?)
             RETURNING id",
        )
        .bind(&now)
        .bind(&now)
        .bind(key)
        .fetch_all(&mut *tx)
        .await?;
        record_all(&mut *tx, EntityType::ScheduleEntry, &entry_ids, Operation::Delete).await?;

        let recipe_ids: Vec<String> = sqlx::query_scalar(
            "UPDATE recipes SET usual_track_id = NULL, updated_at = ?
             WHERE usual_track_id = (SELECT id FROM tracks WHERE key = ?)
             RETURNING id",
        )
        .bind(&now)
        .bind(key)
        .fetch_all(&mut *tx)
        .await?;
        record_all(&mut *tx, EntityType::Recipe, &recipe_ids, Operation::Upsert).await?;

        let track_id: String = sqlx::query_scalar(
            "UPDATE tracks SET deleted_at = ?, updated_at = ? WHERE key = ? RETURNING id",
        )
        .bind(&now)
        .bind(&now)
        .bind(key)
        .fetch_one(&mut *tx)
        .await?;
        record(&mut *tx, EntityType::Track, &track_id, Operation::Delete).await?;
    }

    // `position` is unique among live tracks. Shift every live row out of the 0..n range first
    // so the new positions never collide with a row that has not been moved yet.
    sqlx::query("UPDATE tracks SET position = position + 1000 WHERE deleted_at IS NULL")
        .execute(&mut *tx)
        .await?;

    let family = active_family(&mut *tx).await?;
    for (position, track) in tracks.iter().enumerate() {
        // A new track gets a random id. An existing key keeps its id, which is what makes the
        // seeded tracks the same on every device.
        let track_id: String = sqlx::query_scalar(
            "INSERT INTO tracks (id, key, label, color, position, default_time, visible, repeats_on, family_id, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(key) DO UPDATE SET
                 label = excluded.label,
                 color = excluded.color,
                 position = excluded.position,
                 default_time = excluded.default_time,
                 visible = excluded.visible,
                 repeats_on = excluded.repeats_on,
                 updated_at = excluded.updated_at,
                 deleted_at = NULL
             RETURNING id",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&track.key)
        .bind(track.label.trim())
        .bind(&track.color)
        .bind(position as i64)
        .bind(&track.default_time)
        .bind(track.visible)
        .bind(&track.repeats_on)
        .bind(&family)
        .bind(&now)
        .fetch_one(&mut *tx)
        .await?;
        record(&mut *tx, EntityType::Track, &track_id, Operation::Upsert).await?;
    }

    tx.commit().await?;
    Ok(())
}

fn validate_tracks(tracks: &[TrackInput]) -> Result<(), AppError> {
    for (i, track) in tracks.iter().enumerate() {
        if track.label.trim().is_empty() {
            return Err(AppError::Invalid("a track needs a name".into()));
        }
        if tracks[..i].iter().any(|other| other.key == track.key) {
            return Err(AppError::Invalid(format!("duplicate track key '{}'", track.key)));
        }
    }
    Ok(())
}

/// Planned meals between `start` and `end` (inclusive, `YYYY-MM-DD`), in day then track order.
pub async fn week(pool: &SqlitePool, start: &str, end: &str) -> Result<Vec<PlannedMeal>, AppError> {
    let meals = sqlx::query_as(
        "SELECT se.date, t.key AS track_key, r.id AS recipe_id, r.name, r.minutes, r.rating
         FROM schedule_entries se
         JOIN recipes r ON r.id = se.recipe_id
         JOIN tracks t ON t.id = se.track_id
         WHERE se.date BETWEEN ? AND ?
           AND se.deleted_at IS NULL AND r.deleted_at IS NULL AND t.deleted_at IS NULL
         ORDER BY se.date, t.position",
    )
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await?;
    Ok(meals)
}

/// Puts a recipe in a slot, replacing whatever was there. Setting a cleared slot brings it back.
pub async fn set_entry(
    pool: &SqlitePool,
    date: &str,
    track_key: &str,
    recipe_id: &str,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let now = now(&mut *tx).await?;

    let track_id: String = sqlx::query_scalar("SELECT id FROM tracks WHERE key = ? AND deleted_at IS NULL")
        .bind(track_key)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(AppError::NotFound("track"))?;
    let recipe_live: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM recipes WHERE id = ? AND deleted_at IS NULL)",
    )
    .bind(recipe_id)
    .fetch_one(&mut *tx)
    .await?;
    if !recipe_live {
        return Err(AppError::NotFound("recipe"));
    }

    let family = active_family(&mut *tx).await?;
    let entry_id: String = sqlx::query_scalar(
        "INSERT INTO schedule_entries (id, date, track_id, recipe_id, family_id, updated_at) VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(date, track_id) DO UPDATE SET
             recipe_id = excluded.recipe_id,
             deleted_at = NULL,
             updated_at = excluded.updated_at
         RETURNING id",
    )
    .bind(slot_id(date, &track_id))
    .bind(date)
    .bind(&track_id)
    .bind(recipe_id)
    .bind(&family)
    .bind(&now)
    .fetch_one(&mut *tx)
    .await?;
    record(&mut *tx, EntityType::ScheduleEntry, &entry_id, Operation::Upsert).await?;
    tx.commit().await?;
    Ok(())
}

/// Empties a slot. Clearing an empty slot is not an error.
pub async fn clear_entry(pool: &SqlitePool, date: &str, track_key: &str) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let now = now(&mut *tx).await?;
    let entry_ids: Vec<String> = sqlx::query_scalar(
        "UPDATE schedule_entries SET deleted_at = ?, updated_at = ?
         WHERE date = ? AND deleted_at IS NULL
           AND track_id = (SELECT id FROM tracks WHERE key = ?)
         RETURNING id",
    )
    .bind(&now)
    .bind(&now)
    .bind(date)
    .bind(track_key)
    .fetch_all(&mut *tx)
    .await?;
    record_all(&mut *tx, EntityType::ScheduleEntry, &entry_ids, Operation::Delete).await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_pool;
    use crate::models::RecipeInput;
    use crate::repo::recipes;
    use tauri::async_runtime::block_on;

    async fn recipe(pool: &SqlitePool, name: &str) -> String {
        recipes::create(
            pool,
            &RecipeInput {
                name: name.to_string(),
                url: "https://example.com".to_string(),
                source_domain: "example.com".to_string(),
                minutes: Some(30),
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
        .unwrap()
    }

    #[test]
    fn tracks_are_seeded_in_position_order() {
        block_on(async {
            let pool = test_pool().await;
            let keys: Vec<String> = tracks(&pool).await.unwrap().into_iter().map(|t| t.key).collect();
            assert_eq!(keys, ["breakfast", "breakfast-kids", "lunch", "dinner"]);
        });
    }

    #[test]
    fn set_entry_upserts_one_recipe_per_slot() {
        block_on(async {
            let pool = test_pool().await;
            let oats = recipe(&pool, "oats").await;
            let toast = recipe(&pool, "toast").await;

            set_entry(&pool, "2026-10-26", "breakfast", &oats).await.unwrap();
            set_entry(&pool, "2026-10-26", "breakfast", &toast).await.unwrap();

            let meals = week(&pool, "2026-10-26", "2026-11-01").await.unwrap();
            assert_eq!(meals.len(), 1);
            assert_eq!(meals[0].recipe_id, toast);
            assert_eq!(meals[0].name, "toast");
        });
    }

    #[test]
    fn week_filters_by_range_and_orders_by_track() {
        block_on(async {
            let pool = test_pool().await;
            let soup = recipe(&pool, "soup").await;
            let oats = recipe(&pool, "oats").await;

            set_entry(&pool, "2026-10-26", "dinner", &soup).await.unwrap();
            set_entry(&pool, "2026-10-26", "breakfast", &oats).await.unwrap();
            set_entry(&pool, "2026-11-02", "lunch", &soup).await.unwrap();

            let meals = week(&pool, "2026-10-26", "2026-11-01").await.unwrap();
            let slots: Vec<&str> = meals.iter().map(|m| m.track_key.as_str()).collect();
            assert_eq!(slots, ["breakfast", "dinner"]);
        });
    }

    #[test]
    fn clear_entry_removes_slot_and_is_idempotent() {
        block_on(async {
            let pool = test_pool().await;
            let oats = recipe(&pool, "oats").await;

            set_entry(&pool, "2026-10-26", "breakfast", &oats).await.unwrap();
            clear_entry(&pool, "2026-10-26", "breakfast").await.unwrap();
            clear_entry(&pool, "2026-10-26", "breakfast").await.unwrap();

            assert!(week(&pool, "2026-10-26", "2026-10-26").await.unwrap().is_empty());
        });
    }

    #[test]
    fn deleting_a_recipe_clears_its_slots() {
        block_on(async {
            let pool = test_pool().await;
            let oats = recipe(&pool, "oats").await;
            set_entry(&pool, "2026-10-26", "breakfast", &oats).await.unwrap();

            recipes::delete(&pool, &oats).await.unwrap();

            assert!(week(&pool, "2026-10-26", "2026-10-26").await.unwrap().is_empty());
        });
    }

    fn track_input(key: &str, label: &str) -> TrackInput {
        TrackInput {
            key: key.to_string(),
            label: label.to_string(),
            color: "teal".to_string(),
            default_time: None,
            visible: true,
            repeats_on: "1111111".to_string(),
        }
    }

    fn inputs(tracks: &[crate::models::Track]) -> Vec<TrackInput> {
        tracks
            .iter()
            .map(|t| TrackInput {
                key: t.key.clone(),
                label: t.label.clone(),
                color: t.color.clone(),
                default_time: t.default_time.clone(),
                visible: t.visible,
                repeats_on: t.repeats_on.clone(),
            })
            .collect()
    }

    #[test]
    fn save_tracks_reorders_and_updates_settings() {
        block_on(async {
            let pool = test_pool().await;
            let mut list = inputs(&tracks(&pool).await.unwrap());
            list.reverse();
            list[0].visible = false;
            list[0].default_time = Some("09:00".to_string());
            list[1].repeats_on = "1111100".to_string();

            save_tracks(&pool, &list).await.unwrap();

            let saved = tracks(&pool).await.unwrap();
            let keys: Vec<&str> = saved.iter().map(|t| t.key.as_str()).collect();
            assert_eq!(keys, ["dinner", "lunch", "breakfast-kids", "breakfast"]);
            assert!(!saved[0].visible);
            assert_eq!(saved[0].default_time.as_deref(), Some("09:00"));
            assert_eq!(saved[1].repeats_on, "1111100");
        });
    }

    #[test]
    fn save_tracks_removes_dropped_tracks_and_their_meals() {
        block_on(async {
            let pool = test_pool().await;
            let oats = recipe(&pool, "oats").await;
            set_entry(&pool, "2026-10-26", "lunch", &oats).await.unwrap();

            let mut list = inputs(&tracks(&pool).await.unwrap());
            list.retain(|t| t.key != "lunch");
            save_tracks(&pool, &list).await.unwrap();

            assert!(tracks(&pool).await.unwrap().iter().all(|t| t.key != "lunch"));
            assert!(week(&pool, "2026-10-26", "2026-10-26").await.unwrap().is_empty());
        });
    }

    #[test]
    fn save_tracks_adds_new_track_and_rejects_bad_input() {
        block_on(async {
            let pool = test_pool().await;
            let mut list = inputs(&tracks(&pool).await.unwrap());
            list.push(track_input("snack", "Snack"));
            save_tracks(&pool, &list).await.unwrap();
            assert_eq!(tracks(&pool).await.unwrap().len(), 5);

            let blank = vec![track_input("brunch", "   ")];
            assert!(matches!(save_tracks(&pool, &blank).await, Err(AppError::Invalid(_))));

            let duplicate = vec![track_input("brunch", "Brunch"), track_input("brunch", "Again")];
            assert!(matches!(save_tracks(&pool, &duplicate).await, Err(AppError::Invalid(_))));
        });
    }

    #[test]
    fn set_entry_rejects_unknown_track() {
        block_on(async {
            let pool = test_pool().await;
            let oats = recipe(&pool, "oats").await;

            let result = set_entry(&pool, "2026-10-26", "brunch", &oats).await;
            assert!(matches!(result, Err(AppError::NotFound("track"))));
        });
    }

    #[test]
    fn derived_usage_fields_reflect_schedule() {
        block_on(async {
            let pool = test_pool().await;
            let oats = recipe(&pool, "oats").await;
            let today: String = sqlx::query_scalar("SELECT date('now', 'localtime')")
                .fetch_one(&pool)
                .await
                .unwrap();

            set_entry(&pool, "2020-01-01", "breakfast", &oats).await.unwrap();
            set_entry(&pool, &today, "lunch", &oats).await.unwrap();

            let recipe = recipes::get(&pool, &oats).await.unwrap();
            assert_eq!(recipe.planned_count, 2);
            assert_eq!(recipe.last_used_days_ago, Some(0));
        });
    }

    #[test]
    fn cleared_slot_is_soft_deleted_and_set_again_revives_the_same_row() {
        block_on(async {
            let pool = test_pool().await;
            let oats = recipe(&pool, "oats").await;

            set_entry(&pool, "2026-10-26", "breakfast", &oats).await.unwrap();
            let first: String = sqlx::query_scalar("SELECT id FROM schedule_entries")
                .fetch_one(&pool)
                .await
                .unwrap();
            clear_entry(&pool, "2026-10-26", "breakfast").await.unwrap();

            let deleted: Option<String> =
                sqlx::query_scalar("SELECT deleted_at FROM schedule_entries WHERE id = ?")
                    .bind(&first)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert!(deleted.is_some());

            set_entry(&pool, "2026-10-26", "breakfast", &oats).await.unwrap();
            let rows: Vec<(String, Option<String>)> =
                sqlx::query_as("SELECT id, deleted_at FROM schedule_entries")
                    .fetch_all(&pool)
                    .await
                    .unwrap();
            assert_eq!(rows, vec![(first, None)]);
        });
    }
}
