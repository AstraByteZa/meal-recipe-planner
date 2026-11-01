//! Three devices against a running sync server. Set `SYNC_TEST_SERVER_URL` (for example
//! `http://127.0.0.1:8080`) to run it; without it the test is skipped. Each run uses fresh
//! accounts, so one server can serve many runs.

use sqlx::SqlitePool;
use uuid::Uuid;

use shared_core::wire::FamilyRole;

use crate::db::test_pool;
use crate::models::RecipeInput;
use crate::repo::{recipes, schedule};
use crate::sync::SyncHandle;
use crate::sync::account::{self, session};
use crate::sync::client::Api;
use crate::sync::family;
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

    /// The family check the worker runs at session start. Returns whether anything changed.
    async fn reconcile(&self) -> bool {
        let session = session(&self.pool).await.unwrap().expect("signed in");
        let api = Api::new(&session.server_url, Some(&session.token)).unwrap();
        family::reconcile(&self.pool, &api).await.unwrap()
    }
}

async fn family_id_of(pool: &SqlitePool, recipe_id: &str) -> Option<String> {
    sqlx::query_scalar("SELECT family_id FROM recipes WHERE id = ?")
        .bind(recipe_id)
        .fetch_one(pool)
        .await
        .unwrap()
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
        account::register(&laptop.pool, &laptop.handle, &url, &email, PASSWORD, "").await.unwrap();
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
        account::register(&stranger.pool, &stranger.handle, &url, &format!("e2e-{}@example.test", Uuid::new_v4()), PASSWORD, "")
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

/// Invites, a second device adopting the family without a code, a single-use code, removal that
/// leaves the member's copy private, and deleting a family.
#[test]
fn invites_removal_and_reconcile_follow_the_server() {
    let Ok(url) = std::env::var("SYNC_TEST_SERVER_URL") else {
        eprintln!("skipped: set SYNC_TEST_SERVER_URL to run the family test");
        return;
    };

    tauri::async_runtime::block_on(async {
        let owner = Device::new().await;
        let phone = Device::new().await;
        let tablet = Device::new().await;
        let outsider = Device::new().await;
        let member_email = format!("e2e-{}@example.test", Uuid::new_v4());

        account::register(&owner.pool, &owner.handle, &url, &format!("e2e-{}@example.test", Uuid::new_v4()), PASSWORD, "Owner")
            .await
            .unwrap();
        account::register(&phone.pool, &phone.handle, &url, &member_email, PASSWORD, "Sam").await.unwrap();
        account::register(&outsider.pool, &outsider.handle, &url, &format!("e2e-{}@example.test", Uuid::new_v4()), PASSWORD, "")
            .await
            .unwrap();

        let home = account::family_create(&owner.pool, &owner.handle, "Home").await.unwrap().family.expect("family");
        let shared = recipes::create(&owner.pool, &recipe("Family pancakes", &["Breakfast"])).await.unwrap();
        owner.push().await;

        // The code previews before it is used, and the member joins with it.
        let invite = family::invite_create(&owner.pool).await.unwrap();
        let preview = family::invite_preview(&phone.pool, &invite.code).await.unwrap();
        assert_eq!(preview.family_name, "Home");
        assert_eq!(preview.invited_by, "Owner");
        account::family_join(&phone.pool, &phone.handle, &invite.code).await.unwrap();
        phone.pull().await;
        assert_eq!(family_id_of(&phone.pool, &shared).await.as_deref(), Some(home.id.as_str()));

        // A code works once. Another account using it is refused.
        assert!(account::family_join(&outsider.pool, &outsider.handle, &invite.code).await.is_err());

        let details = family::details(&owner.pool).await.unwrap();
        assert_eq!(details.members.len(), 2);
        let sam = details.members.iter().find(|member| member.role == FamilyRole::Member).expect("the member").user_id;

        // The member's second device learns the family from the server, without a code.
        account::login(&tablet.pool, &tablet.handle, &url, &member_email, PASSWORD).await.unwrap();
        assert!(tablet.reconcile().await, "a second device adopts the one family the account is in");
        tablet.pull().await;
        assert_eq!(family_id_of(&tablet.pool, &shared).await.as_deref(), Some(home.id.as_str()));

        // The owner removes the member. Both of the member's devices detach on reconcile.
        family::member_remove(&owner.pool, &owner.handle, &sam.to_string()).await.unwrap();
        assert!(phone.reconcile().await, "the removed member's device detaches");
        assert!(tablet.reconcile().await, "the second device detaches too");
        let status = account::status(&phone.pool, &phone.handle).await.unwrap();
        assert!(status.family.is_none());
        assert_eq!(status.notice.expect("a removal leaves a notice").family_name, "Home");
        assert_eq!(family_id_of(&phone.pool, &shared).await, None, "the recipe is now private");

        // The member's copy is pushed as their own. The owner keeps the family copy, and an
        // outsider sees neither.
        phone.push().await;
        owner.pull().await;
        assert_eq!(family_id_of(&owner.pool, &shared).await.as_deref(), Some(home.id.as_str()));
        outsider.pull().await;
        let leaked = count(&outsider.pool, "SELECT COUNT(*) FROM recipes WHERE id = ? AND deleted_at IS NULL", &shared).await;
        assert_eq!(leaked, 0, "a removed family's rows stay out of reach of other accounts");

        // Dismissing the notice clears it, and a reconcile with nothing to do changes nothing.
        let cleared = account::dismiss_notice(&phone.pool, &phone.handle).await.unwrap();
        assert!(cleared.notice.is_none());
        assert!(!phone.reconcile().await);

        // The owner deletes the family. The owner keeps the rows and the device has no family.
        family::delete(&owner.pool, &owner.handle).await.unwrap();
        let status = account::status(&owner.pool, &owner.handle).await.unwrap();
        assert!(status.family.is_none());
        assert!(status.notice.is_none(), "deleting your own family is not a removal");
        assert_eq!(family_id_of(&owner.pool, &shared).await, None);
    });
}

