use std::collections::HashMap;

use sqlx::types::Json;
use sqlx::{Sqlite, SqlitePool, Transaction};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{PlanEntry, RecipeDetail, RecipeInput, SavedRecipe};
use crate::repo::{now, tag_id, tag_link_id};
use crate::sync::active_family;
use shared_core::entity::{EntityType, Operation};
use crate::sync::outbox::{record, record_all};

/// Column list shared by the queries below. Only constant fragments are interpolated.
/// The list view ignores the detail-only columns. Soft-deleted schedule rows are left out of
/// the usage counts.
const SELECT_RECIPES: &str = "
    SELECT
        r.id,
        r.name,
        r.url,
        r.source_domain,
        r.rating,
        r.minutes,
        r.servings,
        r.notes,
        ut.key AS usual_track_key,
        r.image_url,
        r.ingredients,
        r.instructions,
        CAST(julianday('now') - julianday(r.created_at) AS INTEGER) AS saved_days_ago,
        (
            SELECT CAST(julianday(date('now', 'localtime')) - julianday(MAX(se.date)) AS INTEGER)
            FROM schedule_entries se
            WHERE se.recipe_id = r.id AND se.deleted_at IS NULL AND se.date <= date('now', 'localtime')
        ) AS last_used_days_ago,
        (
            SELECT COUNT(*) FROM schedule_entries se
            WHERE se.recipe_id = r.id AND se.deleted_at IS NULL
        ) AS planned_count
    FROM recipes r
    LEFT JOIN tracks ut ON ut.id = r.usual_track_id";

pub async fn list(pool: &SqlitePool) -> Result<Vec<SavedRecipe>, AppError> {
    let mut recipes: Vec<SavedRecipe> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{SELECT_RECIPES} WHERE r.deleted_at IS NULL ORDER BY r.created_at, r.id"
    )))
    .fetch_all(pool)
    .await?;
    attach_tags(pool, &mut recipes).await?;
    Ok(recipes)
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<RecipeDetail, AppError> {
    let mut recipe: RecipeDetail = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{SELECT_RECIPES} WHERE r.id = ? AND r.deleted_at IS NULL"
    )))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound("recipe"))?;
    recipe.tags = tags_for(pool, id).await?;
    recipe.history = history_for(pool, id).await?;
    Ok(recipe)
}

pub async fn create(pool: &SqlitePool, input: &RecipeInput) -> Result<String, AppError> {
    let mut tx = pool.begin().await?;
    let id = Uuid::new_v4().to_string();
    let now = now(&mut *tx).await?;
    let family = active_family(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO recipes (id, name, url, source_domain, minutes, rating, servings, notes, usual_track_id,
                              ingredients, instructions, image_url, family_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, (SELECT id FROM tracks WHERE key = ? AND deleted_at IS NULL),
                 ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&input.name)
    .bind(&input.url)
    .bind(&input.source_domain)
    .bind(input.minutes)
    .bind(input.rating)
    .bind(input.servings)
    .bind(&input.notes)
    .bind(&input.usual_track_key)
    .bind(Json(&input.ingredients))
    .bind(Json(&input.instructions))
    .bind(&input.image_url)
    .bind(&family)
    .bind(&now)
    .bind(&now)
    .execute(&mut *tx)
    .await?;
    record(&mut *tx, EntityType::Recipe, &id, Operation::Upsert).await?;
    write_tags(&mut tx, &id, &input.tags).await?;
    tx.commit().await?;
    Ok(id)
}

pub async fn update(pool: &SqlitePool, id: &str, input: &RecipeInput) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let result = sqlx::query(
        "UPDATE recipes
         SET name = ?, url = ?, source_domain = ?, minutes = ?, rating = ?,
             servings = ?, notes = ?,
             usual_track_id = (SELECT id FROM tracks WHERE key = ? AND deleted_at IS NULL),
             ingredients = ?, instructions = ?, image_url = ?,
             updated_at = ?
         WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(&input.name)
    .bind(&input.url)
    .bind(&input.source_domain)
    .bind(input.minutes)
    .bind(input.rating)
    .bind(input.servings)
    .bind(&input.notes)
    .bind(&input.usual_track_key)
    .bind(Json(&input.ingredients))
    .bind(Json(&input.instructions))
    .bind(&input.image_url)
    .bind(now(&mut *tx).await?)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("recipe"));
    }
    record(&mut *tx, EntityType::Recipe, id, Operation::Upsert).await?;
    write_tags(&mut tx, id, &input.tags).await?;
    tx.commit().await?;
    Ok(())
}

/// Soft-deletes the recipe and everything that points at it, so the deletion can sync.
pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let now = now(&mut *tx).await?;
    let result = sqlx::query(
        "UPDATE recipes SET deleted_at = ?, updated_at = ? WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(&now)
    .bind(&now)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("recipe"));
    }
    record(&mut *tx, EntityType::Recipe, id, Operation::Delete).await?;

    let entry_ids: Vec<String> = sqlx::query_scalar(
        "UPDATE schedule_entries SET deleted_at = ?, updated_at = ?
         WHERE recipe_id = ? AND deleted_at IS NULL
         RETURNING id",
    )
    .bind(&now)
    .bind(&now)
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;
    record_all(&mut *tx, EntityType::ScheduleEntry, &entry_ids, Operation::Delete).await?;

    let link_ids: Vec<String> = sqlx::query_scalar(
        "UPDATE recipe_tags SET deleted_at = ?, updated_at = ?
         WHERE recipe_id = ? AND deleted_at IS NULL
         RETURNING id",
    )
    .bind(&now)
    .bind(&now)
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;
    record_all(&mut *tx, EntityType::RecipeTag, &link_ids, Operation::Delete).await?;
    tx.commit().await?;
    Ok(())
}

async fn attach_tags(pool: &SqlitePool, recipes: &mut [SavedRecipe]) -> Result<(), AppError> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT rt.recipe_id, t.name
         FROM recipe_tags rt
         JOIN tags t ON t.id = rt.tag_id
         WHERE rt.deleted_at IS NULL
         ORDER BY t.name",
    )
    .fetch_all(pool)
    .await?;

    let mut by_recipe: HashMap<String, Vec<String>> = HashMap::new();
    for (recipe_id, name) in rows {
        by_recipe.entry(recipe_id).or_default().push(name);
    }
    for recipe in recipes {
        recipe.tags = by_recipe.remove(&recipe.id).unwrap_or_default();
    }
    Ok(())
}

async fn tags_for(pool: &SqlitePool, recipe_id: &str) -> Result<Vec<String>, AppError> {
    let names: Vec<String> = sqlx::query_scalar(
        "SELECT t.name
         FROM recipe_tags rt
         JOIN tags t ON t.id = rt.tag_id
         WHERE rt.recipe_id = ? AND rt.deleted_at IS NULL
         ORDER BY t.name",
    )
    .bind(recipe_id)
    .fetch_all(pool)
    .await?;
    Ok(names)
}

/// Every live schedule slot the recipe is planned in, newest first.
async fn history_for(pool: &SqlitePool, recipe_id: &str) -> Result<Vec<PlanEntry>, AppError> {
    let entries: Vec<PlanEntry> = sqlx::query_as(
        "SELECT se.date, t.key AS track_key, t.label AS track_label
         FROM schedule_entries se
         JOIN tracks t ON t.id = se.track_id
         WHERE se.recipe_id = ? AND se.deleted_at IS NULL AND t.deleted_at IS NULL
         ORDER BY se.date DESC",
    )
    .bind(recipe_id)
    .fetch_all(pool)
    .await?;
    Ok(entries)
}

/// Replaces the recipe's tags. Tags are shared by name (case-insensitive) and get a deterministic
/// id, so two devices that add the same tag converge on one row.
async fn write_tags(
    tx: &mut Transaction<'_, Sqlite>,
    recipe_id: &str,
    tags: &[String],
) -> Result<(), AppError> {
    let now = now(&mut **tx).await?;
    let family = active_family(&mut **tx).await?;
    let stale: Vec<String> = sqlx::query_scalar(
        "UPDATE recipe_tags SET deleted_at = ?, updated_at = ?
         WHERE recipe_id = ? AND deleted_at IS NULL
         RETURNING id",
    )
    .bind(&now)
    .bind(&now)
    .bind(recipe_id)
    .fetch_all(&mut **tx)
    .await?;
    record_all(&mut **tx, EntityType::RecipeTag, &stale, Operation::Delete).await?;

    for name in tags.iter().map(|t| t.trim()).filter(|t| !t.is_empty()) {
        // An existing tag keeps its id, so the RETURNING id is the one to link.
        let tag_id: String = sqlx::query_scalar(
            "INSERT INTO tags (id, name, family_id, updated_at) VALUES (?, ?, ?, ?)
             ON CONFLICT(name) DO UPDATE SET name = excluded.name
             RETURNING id",
        )
        .bind(tag_id(name))
        .bind(name)
        .bind(&family)
        .bind(&now)
        .fetch_one(&mut **tx)
        .await?;
        record(&mut **tx, EntityType::Tag, &tag_id, Operation::Upsert).await?;

        let link_id: String = sqlx::query_scalar(
            "INSERT INTO recipe_tags (id, recipe_id, tag_id, family_id, updated_at) VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(recipe_id, tag_id) DO UPDATE SET deleted_at = NULL, updated_at = excluded.updated_at
             RETURNING id",
        )
        .bind(tag_link_id(recipe_id, &tag_id))
        .bind(recipe_id)
        .bind(&tag_id)
        .bind(&family)
        .bind(&now)
        .fetch_one(&mut **tx)
        .await?;
        record(&mut **tx, EntityType::RecipeTag, &link_id, Operation::Upsert).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_pool;
    use crate::repo::schedule;
    use tauri::async_runtime::block_on;

    fn input(name: &str, tags: &[&str]) -> RecipeInput {
        RecipeInput {
            name: name.to_string(),
            url: format!("https://example.com/{name}"),
            source_domain: "example.com".to_string(),
            minutes: Some(20),
            rating: Some(4),
            servings: Some(2),
            notes: Some("Serve warm".to_string()),
            usual_track_key: Some("breakfast".to_string()),
            ingredients: vec!["oats".to_string(), "milk".to_string()],
            instructions: vec!["Stir.".to_string()],
            image_url: Some("https://example.com/oats.jpg".to_string()),
            tags: tags.iter().map(|t| t.to_string()).collect(),
        }
    }

    #[test]
    fn create_then_get_round_trips_with_tags() {
        block_on(async {
            let pool = test_pool().await;
            let id = create(&pool, &input("oats", &["Breakfast", " Quick ", ""]))
                .await
                .unwrap();

            let recipe = get(&pool, &id).await.unwrap();
            assert_eq!(recipe.name, "oats");
            assert_eq!(recipe.rating, Some(4));
            assert_eq!(recipe.servings, Some(2));
            assert_eq!(recipe.notes.as_deref(), Some("Serve warm"));
            assert_eq!(recipe.usual_track_key.as_deref(), Some("breakfast"));
            assert_eq!(recipe.ingredients, vec!["oats", "milk"]);
            assert_eq!(recipe.instructions, vec!["Stir."]);
            assert_eq!(recipe.image_url.as_deref(), Some("https://example.com/oats.jpg"));
            assert_eq!(recipe.tags, vec!["Breakfast", "Quick"]);
            assert_eq!(recipe.planned_count, 0);
            assert_eq!(recipe.last_used_days_ago, None);
            assert!(recipe.history.is_empty());
        });
    }

    #[test]
    fn get_lists_plan_history_newest_first() {
        block_on(async {
            let pool = test_pool().await;
            let id = create(&pool, &input("stew", &[])).await.unwrap();
            schedule::set_entry(&pool, "2026-10-20", "dinner", &id).await.unwrap();
            schedule::set_entry(&pool, "2026-10-27", "lunch", &id).await.unwrap();

            let history = get(&pool, &id).await.unwrap().history;
            let dates: Vec<&str> = history.iter().map(|e| e.date.as_str()).collect();
            assert_eq!(dates, vec!["2026-10-27", "2026-10-20"]);
            assert_eq!(history[0].track_label, "Lunch");
        });
    }

    #[test]
    fn update_replaces_tags_and_missing_id_is_not_found() {
        block_on(async {
            let pool = test_pool().await;
            let id = create(&pool, &input("soup", &["Lunch"])).await.unwrap();

            update(&pool, &id, &input("soup", &["Dinner"])).await.unwrap();
            assert_eq!(get(&pool, &id).await.unwrap().tags, vec!["Dinner"]);

            let missing = update(&pool, "missing-id", &input("ghost", &[])).await;
            assert!(matches!(missing, Err(AppError::NotFound("recipe"))));
        });
    }

    #[test]
    fn tag_names_are_case_insensitive() {
        block_on(async {
            let pool = test_pool().await;
            create(&pool, &input("a", &["Quick"])).await.unwrap();
            create(&pool, &input("b", &["quick"])).await.unwrap();

            let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tags")
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(count, 1);
        });
    }

    #[test]
    fn delete_removes_recipe_and_missing_id_is_not_found() {
        block_on(async {
            let pool = test_pool().await;
            let id = create(&pool, &input("pie", &["Dinner"])).await.unwrap();

            delete(&pool, &id).await.unwrap();
            assert!(list(&pool).await.unwrap().is_empty());
            assert!(matches!(
                delete(&pool, &id).await,
                Err(AppError::NotFound("recipe"))
            ));
        });
    }

    #[test]
    fn delete_is_soft_and_takes_its_planned_meals_with_it() {
        block_on(async {
            let pool = test_pool().await;
            let id = create(&pool, &input("pie", &["Dinner"])).await.unwrap();
            schedule::set_entry(&pool, "2026-10-26", "dinner", &id).await.unwrap();

            delete(&pool, &id).await.unwrap();

            let deleted: Option<String> = sqlx::query_scalar("SELECT deleted_at FROM recipes WHERE id = ?")
                .bind(&id)
                .fetch_one(&pool)
                .await
                .unwrap();
            assert!(deleted.is_some());
            let live_slots: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM schedule_entries WHERE deleted_at IS NULL",
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(live_slots, 0);
        });
    }
}
