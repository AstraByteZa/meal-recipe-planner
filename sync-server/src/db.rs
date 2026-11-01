use sqlx::PgPool;
use sqlx::migrate::MigrateError;
use sqlx::postgres::PgPoolOptions;

pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new().max_connections(10).connect(database_url).await
}

/// Applies the embedded migrations in `migrations/`. Applied files are never edited.
pub async fn migrate(pool: &PgPool) -> Result<(), MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

/// Applies migrations up to and including `version`. Tests use this to check an upgrade.
pub async fn migrate_to(pool: &PgPool, version: i64) -> Result<(), MigrateError> {
    sqlx::migrate!("./migrations").run_to(version, pool).await
}
