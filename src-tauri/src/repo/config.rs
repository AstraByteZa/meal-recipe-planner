use std::collections::BTreeMap;

use serde_json::Value;
use sqlx::SqlitePool;

use crate::error::AppError;
use crate::repo::{config_id, now};
use shared_core::entity::{EntityType, Operation};
use crate::sync::outbox::record;

/// Every stored setting, by key. Values are parsed JSON, so the frontend's schema decides their types.
pub async fn list(pool: &SqlitePool) -> Result<BTreeMap<String, Value>, AppError> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT key, value FROM config WHERE deleted_at IS NULL")
        .fetch_all(pool)
        .await?;
    rows.into_iter()
        .map(|(key, value)| Ok((key, serde_json::from_str(&value).map_err(invalid_stored_value)?)))
        .collect()
}

/// Saves one setting. The value must be a boolean, number or string, so that every stored
/// setting is a flat value the frontend can check with a schema.
pub async fn set(pool: &SqlitePool, key: &str, value: &Value) -> Result<(), AppError> {
    validate(key, value)?;
    let encoded = serde_json::to_string(value).map_err(invalid_stored_value)?;
    let mut tx = pool.begin().await?;
    let now = now(&mut *tx).await?;
    let config_row: String = sqlx::query_scalar(
        "INSERT INTO config (id, key, value, updated_at) VALUES (?, ?, ?, ?)
         ON CONFLICT(key) DO UPDATE SET
             value = excluded.value,
             updated_at = excluded.updated_at,
             deleted_at = NULL
         RETURNING id",
    )
    .bind(config_id(key))
    .bind(key)
    .bind(encoded)
    .bind(&now)
    .fetch_one(&mut *tx)
    .await?;
    record(&mut *tx, EntityType::Config, &config_row, Operation::Upsert).await?;
    tx.commit().await?;
    Ok(())
}

fn validate(key: &str, value: &Value) -> Result<(), AppError> {
    let well_formed = key.len() <= 64
        && key.starts_with(|c: char| c.is_ascii_lowercase())
        && key.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !well_formed {
        return Err(AppError::Invalid(format!("'{key}' is not a valid setting name")));
    }
    if value.is_array() || value.is_object() || value.is_null() {
        return Err(AppError::Invalid(format!("setting '{key}' must be a boolean, number or text")));
    }
    Ok(())
}

fn invalid_stored_value(error: serde_json::Error) -> AppError {
    AppError::Invalid(format!("stored setting is not valid JSON: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_pool;
    use serde_json::json;
    use tauri::async_runtime::block_on;

    #[test]
    fn starts_empty() {
        block_on(async {
            let pool = test_pool().await;
            assert!(list(&pool).await.unwrap().is_empty());
        });
    }

    #[test]
    fn set_stores_each_scalar_type_and_overwrites() {
        block_on(async {
            let pool = test_pool().await;
            set(&pool, "cook-keep-awake", &json!(true)).await.unwrap();
            set(&pool, "cook-text-scale", &json!("lg")).await.unwrap();
            set(&pool, "cook-timer-volume", &json!(0.5)).await.unwrap();
            set(&pool, "cook-keep-awake", &json!(false)).await.unwrap();

            let config = list(&pool).await.unwrap();
            assert_eq!(config.len(), 3);
            assert_eq!(config["cook-keep-awake"], json!(false));
            assert_eq!(config["cook-text-scale"], json!("lg"));
            assert_eq!(config["cook-timer-volume"], json!(0.5));
        });
    }

    #[test]
    fn rejects_bad_keys_and_non_scalar_values() {
        block_on(async {
            let pool = test_pool().await;
            assert!(matches!(set(&pool, "", &json!(true)).await, Err(AppError::Invalid(_))));
            assert!(matches!(set(&pool, "Cook", &json!(true)).await, Err(AppError::Invalid(_))));
            assert!(matches!(set(&pool, "1st", &json!(true)).await, Err(AppError::Invalid(_))));
            assert!(matches!(set(&pool, "a b", &json!(true)).await, Err(AppError::Invalid(_))));
            assert!(matches!(set(&pool, "ok", &json!(null)).await, Err(AppError::Invalid(_))));
            assert!(matches!(set(&pool, "ok", &json!([1])).await, Err(AppError::Invalid(_))));
            assert!(matches!(set(&pool, "ok", &json!({"a": 1})).await, Err(AppError::Invalid(_))));
            assert!(list(&pool).await.unwrap().is_empty());
        });
    }
}
