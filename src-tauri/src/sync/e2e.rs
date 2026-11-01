//! Three devices against a running sync server. Set `SYNC_TEST_SERVER_URL` (for example
//! `http://127.0.0.1:8080`) to run it; without it the test is skipped. Each run uses fresh
//! accounts, so one server can serve many runs.

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::db::test_pool;
use crate::models::RecipeInput;
use crate::repo::{recipes, schedule};
use crate::sync::SyncHandle;
use crate::sync::account::{self, session};
use crate::sync::client::Api;
use crate::sync::worker::{pull_all, push_all};

const PASSWORD: &str = "correct horse battery";
const DATE: &str = "2026-11-02";

struct Device {
    pool: SqlitePool,
    handle: SyncHandle,
}

impl Device {
    async fn new() -> Self {
        Self { pool: test_pool().await, handle: SyncHandle::default() }
    }

    async fn push(&self) {
        let session = session(&self.pool).await.unwrap().expect("signed in");
        let api = Api::new(&session.server_url, Some(&session.token)).unwrap();
        push_all(&self.pool, &api, &session.client_id).await.unwrap();
    }

    async fn pull(&self) {
        let session = session(&self.pool).await.unwrap().expect("signed in");
        let api = Api::new(&session.server_url, Some(&session.token)).unwrap();
        pull_all(&self.pool, &api, &session.client_id).await.unwrap();
    }
}

fn recipe(name: &str, tags: &[&str]) -> RecipeInput {
    RecipeInput {
        name: name.to_string(),
        url: String::new(),
        source_domain: String::new(),
        minutes: None,
        rating: None,
        servings: None,
        notes: None,
        usual_track_key: None,
        tags: tags.iter().map(|tag| tag.to_string()).collect(),
        ingredients: vec![],
        instructions: vec![],
        image_url: None,
    }
}

/// Counts live rows. The query text is a literal, so it is safe to pass as is.
async fn count(pool: &SqlitePool, sql: &'static str, bind: &str) -> i64 {
    sqlx::query_scalar(sql).bind(bind.to_string()).fetch_one(pool).await.unwrap()
}

#[test]
fn devices_share_recipes_edits_deletes_and_families() {
    let Ok(url) = std::env::var("SYNC_TEST_SERVER_URL") else {
        eprintln!("skipped: set SYNC_TEST_SERVER_URL to run the two-device sync test");
        return;
    };

    tauri::async_runtime::block_on(async {
        let email = format!("e2e-{}@example.test", Uuid::new_v4());
        let laptop = Device::new().await;
        let phone = Device::new().await;
        let stranger = Device::new().await;

        // The laptop signs up, makes a private recipe in a slot, and pushes it.
        account::register(&laptop.pool, &laptop.handle, &url, &email, PASSWORD).await.unwrap();
        let oats = recipes::create(&laptop.pool, &recipe("Overnight oats", &["Breakfast"])).await.unwrap();
        schedule::set_entry(&laptop.pool, DATE, "dinner", &oats).await.unwrap();
        laptop.push().await;

        // The phone signs in to the same account and gets the recipe and its slot.
        account::login(&phone.pool, &phone.handle, &url, &email, PASSWORD).await.unwrap();
        phone.pull().await;
        let name: String = sqlx::query_scalar("SELECT name FROM recipes WHERE id = ?")
            .bind(&oats)
            .fetch_one(&phone.pool)
            .await
            .unwrap();
        assert_eq!(name, "Overnight oats");
        let slots = count(
            &phone.pool,
            "SELECT COUNT(*) FROM schedule_entries WHERE recipe_id = ? AND deleted_at IS NULL",
            &oats,
        )
        .await;
        assert_eq!(slots, 1, "the phone should see the planned dinner");

        // The phone renames the recipe. The laptop pulls and sees the new name.
        recipes::update(&phone.pool, &oats, &recipe("Oats with berries", &["Breakfast"])).await.unwrap();
        phone.push().await;
        laptop.pull().await;
        let name: String = sqlx::query_scalar("SELECT name FROM recipes WHERE id = ?")
            .bind(&oats)
            .fetch_one(&laptop.pool)
            .await
            .unwrap();
        assert_eq!(name, "Oats with berries");

        // Both devices create the same tag, with different capitals. It must stay one row.
        let soup = recipes::create(&phone.pool, &recipe("Lentil soup", &["Quick"])).await.unwrap();
        phone.push().await;
        let salad = recipes::create(&laptop.pool, &recipe("Green salad", &["quick"])).await.unwrap();
        laptop.push().await;
        laptop.pull().await;
        phone.pull().await;
        for device in [&laptop, &phone] {
            let tags: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tags WHERE deleted_at IS NULL AND lower(name) = 'quick'")
                .fetch_one(&device.pool)
                .await
                .unwrap();
            assert_eq!(tags, 1, "the same tag must not be duplicated");
        }
        assert!(recipes::get(&laptop.pool, &soup).await.is_ok());
        assert!(recipes::get(&phone.pool, &salad).await.is_ok());

        // The laptop deletes the oats recipe. Its planned dinner goes with it on the phone.
        recipes::delete(&laptop.pool, &oats).await.unwrap();
        laptop.push().await;
        phone.pull().await;
        let live = count(&phone.pool, "SELECT COUNT(*) FROM recipes WHERE id = ? AND deleted_at IS NULL", &oats).await;
        assert_eq!(live, 0, "the deleted recipe should be gone on the phone");
        let slots = count(
            &phone.pool,
            "SELECT COUNT(*) FROM schedule_entries WHERE recipe_id = ? AND deleted_at IS NULL",
            &oats,
        )
        .await;
        assert_eq!(slots, 0, "its planned meals should be gone too");

        // A second account must not see the private recipes of the first.
        account::register(&stranger.pool, &stranger.handle, &url, &format!("e2e-{}@example.test", Uuid::new_v4()), PASSWORD)
            .await
            .unwrap();
        stranger.pull().await;
        let leaked = count(&stranger.pool, "SELECT COUNT(*) FROM recipes WHERE id = ? AND deleted_at IS NULL", &soup).await;
        assert_eq!(leaked, 0, "another account must not receive private recipes");

        // The laptop creates a family and shares a recipe through it. The phone joins and
        // pulls the family's history. The stranger still sees nothing.
        let family = account::family_create(&laptop.pool, &laptop.handle, "Home").await.unwrap().family.expect("family");
        let shared = recipes::create(&laptop.pool, &recipe("Family pancakes", &["Breakfast"])).await.unwrap();
        laptop.push().await;

        let owner = session(&laptop.pool).await.unwrap().expect("the laptop is signed in");
        let invite = Api::new(&owner.server_url, Some(&owner.token)).unwrap().create_invite(&family.id).await.unwrap();
        account::family_join(&phone.pool, &phone.handle, &invite.code).await.unwrap();
        phone.pull().await;
        let family_id: Option<String> = sqlx::query_scalar("SELECT family_id FROM recipes WHERE id = ?")
            .bind(&shared)
            .fetch_one(&phone.pool)
            .await
            .unwrap();
        assert_eq!(family_id.as_deref(), Some(family.id.as_str()));

        stranger.pull().await;
        let leaked = count(&stranger.pool, "SELECT COUNT(*) FROM recipes WHERE id = ? AND deleted_at IS NULL", &shared).await;
        assert_eq!(leaked, 0, "family recipes must stay within the family");
    });
}
