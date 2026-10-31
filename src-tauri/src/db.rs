use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};
use tauri::{AppHandle, Manager};

use crate::error::AppError;

/// Opens the app database in the app data directory and applies pending migrations.
pub async fn init(app: &AppHandle) -> Result<SqlitePool, AppError> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;

    let options = SqliteConnectOptions::new()
        .filename(dir.join("planner.db"))
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal);

    open(options, 5).await
}

async fn open(options: SqliteConnectOptions, max_connections: u32) -> Result<SqlitePool, AppError> {
    let pool = SqlitePoolOptions::new()
        .max_connections(max_connections)
        .connect_with(options)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

/// In-memory database with migrations applied. One connection, because each in-memory
/// connection is its own database.
#[cfg(test)]
pub async fn test_pool() -> SqlitePool {
    use std::str::FromStr;

    let options = SqliteConnectOptions::from_str("sqlite::memory:")
        .unwrap()
        .foreign_keys(true);
    open(options, 1).await.unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::async_runtime::block_on;

    #[test]
    fn migrations_are_applied_once_on_a_file_database() {
        let path = std::env::temp_dir().join(format!("planner-db-test-{}.db", std::process::id()));
        let options = || {
            SqliteConnectOptions::new()
                .filename(&path)
                .create_if_missing(true)
                .foreign_keys(true)
        };

        block_on(async {
            open(options(), 1).await.unwrap().close().await;
            let pool = open(options(), 1).await.unwrap();
            let tracks: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tracks")
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(tracks, 4);
            pool.close().await;
        });

        let _ = std::fs::remove_file(&path);
    }

    /// Builds a pre-0007 database with real rows, runs 0007 on it, and checks that the rows and
    /// their links survive with UUID ids.
    #[test]
    fn migration_0007_keeps_data_and_links_on_uuid_ids() {
        use std::str::FromStr;

        block_on(async {
            let pool = sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(
                    SqliteConnectOptions::from_str("sqlite::memory:")
                        .unwrap()
                        .foreign_keys(true),
                )
                .await
                .unwrap();

            for sql in [
                include_str!("../migrations/0001_init.sql"),
                include_str!("../migrations/0002_tracks_settings.sql"),
                include_str!("../migrations/0003_recipe_details.sql"),
                include_str!("../migrations/0004_onboarding.sql"),
                include_str!("../migrations/0005_recipe_import.sql"),
                include_str!("../migrations/0006_config.sql"),
            ] {
                sqlx::raw_sql(sql).execute(&pool).await.unwrap();
            }
            sqlx::raw_sql(
                "INSERT INTO tracks (key, label, color, position) VALUES ('snack', 'Snack', 'teal', 4);
                 INSERT INTO recipes (name, url, source_domain, minutes, rating, usual_track_key, ingredients, instructions)
                     VALUES ('oats', 'https://example.com/oats', 'example.com', 10, 4, 'breakfast', '[\"oats\"]', '[\"Stir.\"]');
                 INSERT INTO recipes (name, url, source_domain) VALUES ('stew', '', '');
                 INSERT INTO tags (name) VALUES ('Quick');
                 INSERT INTO recipe_tags (recipe_id, tag_id) VALUES (1, 1);
                 INSERT INTO schedule_entries (date, track_key, recipe_id) VALUES ('2026-10-26', 'breakfast', 1);
                 UPDATE settings SET household_size = 4, onboarded = 1;
                 INSERT INTO household_diets (diet) VALUES ('vegan');
                 INSERT INTO config (key, value) VALUES ('cook-keep-awake', 'true');",
            )
            .execute(&pool)
            .await
            .unwrap();

            sqlx::raw_sql(include_str!("../migrations/0007_sync_ids.sql"))
                .execute(&pool)
                .await
                .unwrap();

            let (oats_id, usual_track): (String, Option<String>) = sqlx::query_as(
                "SELECT r.id, t.key FROM recipes r LEFT JOIN tracks t ON t.id = r.usual_track_id
                 WHERE r.name = 'oats'",
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(oats_id.len(), 36);
            assert_eq!(usual_track.as_deref(), Some("breakfast"));

            let tags: Vec<(String, String)> = sqlx::query_as(
                "SELECT rt.recipe_id, t.name FROM recipe_tags rt JOIN tags t ON t.id = rt.tag_id",
            )
            .fetch_all(&pool)
            .await
            .unwrap();
            assert_eq!(tags, vec![(oats_id.clone(), "Quick".to_string())]);

            let slot: (String, String, String) = sqlx::query_as(
                "SELECT se.date, t.key, se.recipe_id FROM schedule_entries se JOIN tracks t ON t.id = se.track_id",
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(slot, ("2026-10-26".into(), "breakfast".into(), oats_id));

            let snack: String = sqlx::query_scalar("SELECT id FROM tracks WHERE key = 'snack'")
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(snack.len(), 36);

            let (household, onboarded): (i64, i64) =
                sqlx::query_as("SELECT household_size, onboarded FROM settings")
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!((household, onboarded), (4, 1));

            let diets: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM household_diets WHERE diet = 'vegan'")
                .fetch_one(&pool)
                .await
                .unwrap();
            let config: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM config WHERE key = 'cook-keep-awake'")
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!((diets, config), (1, 1));

            let broken: Vec<(String, i64, String, i64)> =
                sqlx::query_as("PRAGMA foreign_key_check").fetch_all(&pool).await.unwrap();
            assert!(broken.is_empty());
        });
    }
}
