//! Upgrading a database that already has families from 0001 to 0002.

mod common;

use common::empty_database;
use sync_server::db;
use uuid::Uuid;

#[tokio::test]
async fn upgrade_makes_the_earliest_joiner_the_owner() {
    let Some(pool) = empty_database().await else { return };
    db::migrate_to(&pool, 1).await.expect("apply 0001 only");

    let family = Uuid::new_v4();
    let first = Uuid::new_v4();
    let second = Uuid::new_v4();
    for (id, email) in [(first, "first@example.com"), (second, "second@example.com")] {
        sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, 'hash')")
            .bind(id)
            .bind(email)
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query("INSERT INTO families (id, name, invite_code) VALUES ($1, 'Old', 'OLDCODE1')")
        .bind(family)
        .execute(&pool)
        .await
        .unwrap();
    // The later joiner goes in first, so the owner comes from the timestamps and not insert order.
    sqlx::query("INSERT INTO family_members (family_id, user_id, joined_at) VALUES ($1, $2, now() - interval '1 hour')")
        .bind(family)
        .bind(second)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO family_members (family_id, user_id, joined_at) VALUES ($1, $2, now() - interval '2 hours')")
        .bind(family)
        .bind(first)
        .execute(&pool)
        .await
        .unwrap();

    db::migrate(&pool).await.expect("apply 0002");

    let roles: Vec<(Uuid, String)> =
        sqlx::query_as("SELECT user_id, role FROM family_members WHERE family_id = $1 ORDER BY joined_at")
            .bind(family)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(roles, vec![(first, "owner".to_string()), (second, "member".to_string())]);

    let old_column: Option<String> = sqlx::query_scalar(
        "SELECT column_name FROM information_schema.columns WHERE table_name = 'families' AND column_name = 'invite_code'",
    )
    .fetch_optional(&pool)
    .await
    .unwrap();
    assert!(old_column.is_none(), "the per-family invite code is dropped");
}
