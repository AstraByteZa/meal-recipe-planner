use sqlx::types::Json;
use sqlx::SqlitePool;

use crate::error::AppError;
use crate::models::{Settings, SettingsInput};
use crate::repo::{diet_id, now};
use shared_core::entity::{EntityType, Operation};
use crate::sync::outbox::{record, record_all};

/// Dietary needs the household can pick. Must match the CHECK in migration 0007.
const DIETS: [&str; 6] = ["vegetarian", "vegan", "pescatarian", "gluten-free", "dairy-free", "nut-free"];

/// The one household row. Fixed so every device addresses the same row (see migration 0007).
pub const SETTINGS_ID: &str = "00000000-0000-4000-8000-000000000010";

pub async fn get(pool: &SqlitePool) -> Result<Settings, AppError> {
    let mut settings: Settings = sqlx::query_as(
        "SELECT household_size, week_starts_on, onboarded FROM settings
         WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(SETTINGS_ID)
    .fetch_one(pool)
    .await?;
    settings.dietary = sqlx::query_scalar(
        "SELECT diet FROM household_diets WHERE deleted_at IS NULL ORDER BY diet",
    )
    .fetch_all(pool)
    .await?;
    Ok(settings)
}

/// Replaces the household size, week start and dietary needs. Onboarding state is untouched.
pub async fn save(pool: &SqlitePool, input: &SettingsInput) -> Result<(), AppError> {
    validate(input)?;

    let mut tx = pool.begin().await?;
    let now = now(&mut *tx).await?;
    sqlx::query(
        "UPDATE settings SET household_size = ?, week_starts_on = ?, updated_at = ? WHERE id = ?",
    )
    .bind(input.household_size)
    .bind(&input.week_starts_on)
    .bind(&now)
    .bind(SETTINGS_ID)
    .execute(&mut *tx)
    .await?;
    record(&mut *tx, EntityType::Settings, SETTINGS_ID, Operation::Upsert).await?;

    // Diets not in the list are soft-deleted. Diets in the list are kept, or revived if they were removed.
    let removed: Vec<String> = sqlx::query_scalar(
        "UPDATE household_diets SET deleted_at = ?, updated_at = ?
         WHERE deleted_at IS NULL AND diet NOT IN (SELECT value FROM json_each(?))
         RETURNING id",
    )
    .bind(&now)
    .bind(&now)
    .bind(Json(&input.dietary))
    .fetch_all(&mut *tx)
    .await?;
    record_all(&mut *tx, EntityType::HouseholdDiet, &removed, Operation::Delete).await?;
    for diet in &input.dietary {
        // Nothing is returned when the diet is already live, and then nothing changed.
        let revived: Option<String> = sqlx::query_scalar(
            "INSERT INTO household_diets (id, diet, updated_at) VALUES (?, ?, ?)
             ON CONFLICT(diet) DO UPDATE SET deleted_at = NULL, updated_at = excluded.updated_at
             WHERE household_diets.deleted_at IS NOT NULL
             RETURNING id",
        )
        .bind(diet_id(diet))
        .bind(diet)
        .bind(&now)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(diet_row) = revived {
            record(&mut *tx, EntityType::HouseholdDiet, &diet_row, Operation::Upsert).await?;
        }
    }
    tx.commit().await?;
    Ok(())
}

/// Device flag, so it does not bump `updated_at` and is not synced.
pub async fn complete_onboarding(pool: &SqlitePool) -> Result<(), AppError> {
    sqlx::query("UPDATE settings SET onboarded = 1 WHERE id = ?")
        .bind(SETTINGS_ID)
        .execute(pool)
        .await?;
    Ok(())
}

fn validate(input: &SettingsInput) -> Result<(), AppError> {
    if !(1..=99).contains(&input.household_size) {
        return Err(AppError::Invalid("household size must be between 1 and 99".into()));
    }
    if !matches!(input.week_starts_on.as_str(), "monday" | "sunday") {
        return Err(AppError::Invalid("the week must start on monday or sunday".into()));
    }
    if let Some(diet) = input.dietary.iter().find(|diet| !DIETS.contains(&diet.as_str())) {
        return Err(AppError::Invalid(format!("unknown dietary need '{diet}'")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_pool;
    use tauri::async_runtime::block_on;

    fn input(household_size: i64, dietary: &[&str]) -> SettingsInput {
        SettingsInput {
            household_size,
            week_starts_on: "sunday".to_string(),
            dietary: dietary.iter().map(|d| d.to_string()).collect(),
        }
    }

    #[test]
    fn defaults_are_seeded_and_not_onboarded() {
        block_on(async {
            let pool = test_pool().await;
            let settings = get(&pool).await.unwrap();
            assert_eq!(settings.household_size, 2);
            assert_eq!(settings.week_starts_on, "monday");
            assert!(!settings.onboarded);
            assert!(settings.dietary.is_empty());
        });
    }

    #[test]
    fn save_replaces_household_and_dietary_needs() {
        block_on(async {
            let pool = test_pool().await;
            save(&pool, &input(4, &["vegan", "nut-free"])).await.unwrap();
            save(&pool, &input(3, &["vegetarian", "vegetarian"])).await.unwrap();

            let settings = get(&pool).await.unwrap();
            assert_eq!(settings.household_size, 3);
            assert_eq!(settings.week_starts_on, "sunday");
            assert_eq!(settings.dietary, ["vegetarian"]);
        });
    }

    #[test]
    fn save_keeps_onboarding_state() {
        block_on(async {
            let pool = test_pool().await;
            complete_onboarding(&pool).await.unwrap();
            save(&pool, &input(2, &[])).await.unwrap();
            assert!(get(&pool).await.unwrap().onboarded);
        });
    }

    #[test]
    fn save_rejects_invalid_input() {
        block_on(async {
            let pool = test_pool().await;
            assert!(matches!(save(&pool, &input(0, &[])).await, Err(AppError::Invalid(_))));
            assert!(matches!(save(&pool, &input(100, &[])).await, Err(AppError::Invalid(_))));
            assert!(matches!(save(&pool, &input(2, &["keto"])).await, Err(AppError::Invalid(_))));

            let mut bad_week = input(2, &[]);
            bad_week.week_starts_on = "friday".to_string();
            assert!(matches!(save(&pool, &bad_week).await, Err(AppError::Invalid(_))));
        });
    }
}
