//! Family roles, invite codes, membership changes and the notices they send.

mod common;

use common::{TestServer, at, new_id, spawn, upsert};
use serde_json::{Value, json};

/// Every unusable code gets this message, so a caller cannot tell revoked from mistyped.
const INVALID_CODE: &str = "That invite code is not valid or has expired";

/// Creates an invite as the owner and returns the whole response body.
async fn invite(server: &TestServer, owner: &str, family: &str) -> Value {
    let response = server
        .http
        .post(server.url(&format!("/family/{family}/invites")))
        .bearer_auth(owner)
        .send()
        .await
        .expect("invite request");
    assert_eq!(response.status(), 201, "owner creates an invite");
    response.json().await.expect("invite body")
}

async fn join(server: &TestServer, token: &str, code: &str) -> reqwest::Response {
    server
        .http
        .post(server.url("/family/join"))
        .bearer_auth(token)
        .json(&json!({ "invite_code": code }))
        .send()
        .await
        .expect("join request")
}

async fn details(server: &TestServer, token: &str, family: &str) -> reqwest::Response {
    server
        .http
        .get(server.url(&format!("/family/{family}")))
        .bearer_auth(token)
        .send()
        .await
        .expect("details request")
}

async fn error_of(response: reqwest::Response) -> String {
    response.json::<Value>().await.expect("error body")["error"].as_str().expect("error text").to_string()
}

#[tokio::test]
async fn owner_invites_members_and_the_family_lists_their_roles() {
    let Some(server) = spawn(true).await else { return };
    let owner = server.register("owner@example.com").await;
    let guest = server.register("guest@example.com").await;
    let family = server.create_family(&owner.token, "The Morgans").await;
    let family_id = family["id"].as_str().unwrap().to_string();
    assert_eq!(family["role"], "owner");

    let created = invite(&server, &owner.token, &family_id).await;
    let code = created["code"].as_str().unwrap().to_string();
    assert_eq!(code.len(), 9, "codes are shown as XXXX-XXXX");
    assert_eq!(&code[4..5], "-");

    // Codes ignore case, spaces and the dash.
    let joined = join(&server, &guest.token, &format!(" {} ", code.replace('-', "").to_lowercase())).await;
    assert_eq!(joined.status(), 200);
    assert_eq!(joined.json::<Value>().await.unwrap()["role"], "member");

    let owner_view: Value = details(&server, &owner.token, &family_id).await.json().await.unwrap();
    assert_eq!(owner_view["family"]["role"], "owner");
    assert_eq!(owner_view["members"][0]["name"], "owner", "no display name falls back to the email");
    assert_eq!(owner_view["members"][0]["role"], "owner");
    assert_eq!(owner_view["members"][1]["email"], "guest@example.com");
    assert_eq!(owner_view["members"][1]["role"], "member");
    assert_eq!(owner_view["invites"], json!([]), "the used code is no longer pending");

    let guest_view: Value = details(&server, &guest.token, &family_id).await.json().await.unwrap();
    assert_eq!(guest_view["family"]["role"], "member");
    assert_eq!(guest_view["invites"], json!([]), "members never see invite codes");
}

#[tokio::test]
async fn a_code_works_once_and_every_bad_code_gets_the_same_message() {
    let Some(server) = spawn(true).await else { return };
    let owner = server.register("owner@example.com").await;
    let used = server.register("used@example.com").await;
    let revoked = server.register("revoked@example.com").await;
    let expired = server.register("expired@example.com").await;
    let outsider = server.register("outsider@example.com").await;
    let family = server.create_family(&owner.token, "Once").await;
    let family_id = family["id"].as_str().unwrap().to_string();

    let first = invite(&server, &owner.token, &family_id).await;
    assert_eq!(join(&server, &used.token, first["code"].as_str().unwrap()).await.status(), 200);
    let second_use = join(&server, &outsider.token, first["code"].as_str().unwrap()).await;
    assert_eq!(second_use.status(), 404);
    assert_eq!(error_of(second_use).await, INVALID_CODE);

    let to_revoke = invite(&server, &owner.token, &family_id).await;
    let revoke = server
        .http
        .delete(server.url(&format!("/family/{family_id}/invites/{}", to_revoke["id"].as_str().unwrap())))
        .bearer_auth(&owner.token)
        .send()
        .await
        .unwrap();
    assert_eq!(revoke.status(), 204);
    let revoked_join = join(&server, &revoked.token, to_revoke["code"].as_str().unwrap()).await;
    assert_eq!(error_of(revoked_join).await, INVALID_CODE);
    let revoke_again = server
        .http
        .delete(server.url(&format!("/family/{family_id}/invites/{}", to_revoke["id"].as_str().unwrap())))
        .bearer_auth(&owner.token)
        .send()
        .await
        .unwrap();
    assert_eq!(revoke_again.status(), 404);

    let to_expire = invite(&server, &owner.token, &family_id).await;
    let plain_code = to_expire["code"].as_str().unwrap().replace('-', "");
    sqlx::query("UPDATE family_invites SET expires_at = now() - interval '1 day' WHERE code = $1")
        .bind(&plain_code)
        .execute(&server.pool)
        .await
        .unwrap();
    let expired_join = join(&server, &expired.token, to_expire["code"].as_str().unwrap()).await;
    assert_eq!(error_of(expired_join).await, INVALID_CODE);

    let garbage = join(&server, &outsider.token, "not-a-code").await;
    assert_eq!(error_of(garbage).await, INVALID_CODE);
}

#[tokio::test]
async fn an_existing_member_does_not_use_up_a_code() {
    let Some(server) = spawn(true).await else { return };
    let owner = server.register("owner@example.com").await;
    let guest = server.register("guest@example.com").await;
    let newcomer = server.register("newcomer@example.com").await;
    let family = server.create_family(&owner.token, "Keep").await;
    let family_id = family["id"].as_str().unwrap().to_string();

    let first = invite(&server, &owner.token, &family_id).await;
    assert_eq!(join(&server, &guest.token, first["code"].as_str().unwrap()).await.status(), 200);

    let second = invite(&server, &owner.token, &family_id).await;
    let again = join(&server, &guest.token, second["code"].as_str().unwrap()).await;
    assert_eq!(again.status(), 200, "a member who uses a code gets the family back");
    assert_eq!(again.json::<Value>().await.unwrap()["role"], "member");

    let pending: Value = details(&server, &owner.token, &family_id).await.json().await.unwrap();
    assert_eq!(pending["invites"].as_array().unwrap().len(), 1, "the code is still pending");
    assert_eq!(join(&server, &newcomer.token, second["code"].as_str().unwrap()).await.status(), 200);
}

#[tokio::test]
async fn only_the_owner_manages_invites_and_the_name() {
    let Some(server) = spawn(true).await else { return };
    let owner = server.register("owner@example.com").await;
    let guest = server.register("guest@example.com").await;
    let outsider = server.register("outsider@example.com").await;
    let family = server.create_family(&owner.token, "Owners").await;
    let family_id = family["id"].as_str().unwrap().to_string();
    let created = invite(&server, &owner.token, &family_id).await;
    server.join_family(&owner.token, &family_id, &guest.token).await;

    let member_invite = server
        .http
        .post(server.url(&format!("/family/{family_id}/invites")))
        .bearer_auth(&guest.token)
        .send()
        .await
        .unwrap();
    assert_eq!(member_invite.status(), 403);

    let member_revoke = server
        .http
        .delete(server.url(&format!("/family/{family_id}/invites/{}", created["id"].as_str().unwrap())))
        .bearer_auth(&guest.token)
        .send()
        .await
        .unwrap();
    assert_eq!(member_revoke.status(), 403);

    let member_rename = server
        .http
        .patch(server.url(&format!("/family/{family_id}")))
        .bearer_auth(&guest.token)
        .json(&json!({ "name": "Mine now" }))
        .send()
        .await
        .unwrap();
    assert_eq!(member_rename.status(), 403);

    // Someone outside the family learns nothing about it, not even that it exists.
    let outsider_view = details(&server, &outsider.token, &family_id).await;
    assert_eq!(outsider_view.status(), 404);
    let outsider_delete = server
        .http
        .delete(server.url(&format!("/family/{family_id}")))
        .bearer_auth(&outsider.token)
        .send()
        .await
        .unwrap();
    assert_eq!(outsider_delete.status(), 404);

    let renamed = server
        .http
        .patch(server.url(&format!("/family/{family_id}")))
        .bearer_auth(&owner.token)
        .json(&json!({ "name": "The Morgans" }))
        .send()
        .await
        .unwrap();
    assert_eq!(renamed.status(), 200);
    let view: Value = details(&server, &guest.token, &family_id).await.json().await.unwrap();
    assert_eq!(view["family"]["name"], "The Morgans");
}

#[tokio::test]
async fn pending_invites_are_capped_at_ten() {
    let Some(server) = spawn(true).await else { return };
    let owner = server.register("owner@example.com").await;
    let family = server.create_family(&owner.token, "Busy").await;
    let family_id = family["id"].as_str().unwrap().to_string();

    let mut ids = Vec::new();
    for _ in 0..10 {
        ids.push(invite(&server, &owner.token, &family_id).await["id"].as_str().unwrap().to_string());
    }
    let eleventh = server
        .http
        .post(server.url(&format!("/family/{family_id}/invites")))
        .bearer_auth(&owner.token)
        .send()
        .await
        .unwrap();
    assert_eq!(eleventh.status(), 409);

    // Revoking one frees a slot.
    let revoke = server
        .http
        .delete(server.url(&format!("/family/{family_id}/invites/{}", ids[0])))
        .bearer_auth(&owner.token)
        .send()
        .await
        .unwrap();
    assert_eq!(revoke.status(), 204);
    invite(&server, &owner.token, &family_id).await;
}

#[tokio::test]
async fn owner_cannot_leave_and_transfer_hands_over_the_role() {
    let Some(server) = spawn(true).await else { return };
    let owner = server.register("owner@example.com").await;
    let heir = server.register("heir@example.com").await;
    let family = server.create_family(&owner.token, "Handover").await;
    let family_id = family["id"].as_str().unwrap().to_string();
    server.join_family(&owner.token, &family_id, &heir.token).await;

    let owner_leaves = server
        .http
        .delete(server.url(&format!("/family/{family_id}/members/{}", owner.user_id)))
        .bearer_auth(&owner.token)
        .send()
        .await
        .unwrap();
    assert_eq!(owner_leaves.status(), 409);

    let stranger = server.register("stranger@example.com").await;
    let to_stranger = server
        .http
        .post(server.url(&format!("/family/{family_id}/owner")))
        .bearer_auth(&owner.token)
        .json(&json!({ "user_id": stranger.user_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(to_stranger.status(), 404, "only members can be made owner");

    let transfer = server
        .http
        .post(server.url(&format!("/family/{family_id}/owner")))
        .bearer_auth(&owner.token)
        .json(&json!({ "user_id": heir.user_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(transfer.status(), 204);

    let old_owner: Value = details(&server, &owner.token, &family_id).await.json().await.unwrap();
    assert_eq!(old_owner["family"]["role"], "member");
    let new_owner: Value = details(&server, &heir.token, &family_id).await.json().await.unwrap();
    assert_eq!(new_owner["family"]["role"], "owner");

    // The old owner can now leave.
    let leave = server
        .http
        .delete(server.url(&format!("/family/{family_id}/members/{}", owner.user_id)))
        .bearer_auth(&owner.token)
        .send()
        .await
        .unwrap();
    assert_eq!(leave.status(), 204);
    let families: Value = server
        .http
        .get(server.url("/family"))
        .bearer_auth(&owner.token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(families, json!([]));
}

#[tokio::test]
async fn removed_members_lose_shared_rows_and_keep_their_private_ones() {
    let Some(server) = spawn(true).await else { return };
    let owner = server.register("owner@example.com").await;
    let guest = server.register("guest@example.com").await;
    let family = server.create_family(&owner.token, "Kitchen").await;
    let family_id = family["id"].as_str().unwrap().to_string();
    server.join_family(&owner.token, &family_id, &guest.token).await;

    server
        .push_ok(&owner.token, "phone", json!([upsert("recipe", &new_id(), Some(&family_id), &at(100), "Ours")]))
        .await;
    server
        .push_ok(&guest.token, "tablet", json!([upsert("recipe", &new_id(), None, &at(90), "Mine")]))
        .await;

    let remove = server
        .http
        .delete(server.url(&format!("/family/{family_id}/members/{}", guest.user_id)))
        .bearer_auth(&owner.token)
        .send()
        .await
        .unwrap();
    assert_eq!(remove.status(), 204);

    let later = server
        .push_ok(&guest.token, "tablet", json!([upsert("recipe", &new_id(), Some(&family_id), &at(10), "Late")]))
        .await;
    assert_eq!(later, ["rejected"], "a removed member cannot push to the family any more");

    let names: Vec<String> = server.pull(&guest.token, 0).await["ops"]
        .as_array()
        .unwrap()
        .iter()
        .map(|op| op["payload"]["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, ["Mine"]);
}

#[tokio::test]
async fn deleting_a_family_keeps_the_members_private_rows() {
    let Some(server) = spawn(true).await else { return };
    let owner = server.register("owner@example.com").await;
    let guest = server.register("guest@example.com").await;
    let family = server.create_family(&owner.token, "Gone").await;
    let family_id = family["id"].as_str().unwrap().to_string();
    server.join_family(&owner.token, &family_id, &guest.token).await;

    server
        .push_ok(&owner.token, "phone", json!([upsert("recipe", &new_id(), Some(&family_id), &at(100), "Ours")]))
        .await;
    server
        .push_ok(&guest.token, "tablet", json!([upsert("recipe", &new_id(), None, &at(90), "Mine")]))
        .await;

    let member_delete = server
        .http
        .delete(server.url(&format!("/family/{family_id}")))
        .bearer_auth(&guest.token)
        .send()
        .await
        .unwrap();
    assert_eq!(member_delete.status(), 403);

    let delete = server
        .http
        .delete(server.url(&format!("/family/{family_id}")))
        .bearer_auth(&owner.token)
        .send()
        .await
        .unwrap();
    assert_eq!(delete.status(), 204);

    let guest_names: Vec<String> = server.pull(&guest.token, 0).await["ops"]
        .as_array()
        .unwrap()
        .iter()
        .map(|op| op["payload"]["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(guest_names, ["Mine"]);
    let owner_families: Value = server
        .http
        .get(server.url("/family"))
        .bearer_auth(&owner.token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(owner_families, json!([]));
}

#[tokio::test]
async fn failed_join_attempts_are_throttled() {
    let Some(server) = spawn(true).await else { return };
    let owner = server.register("owner@example.com").await;
    let guesser = server.register("guesser@example.com").await;
    let family = server.create_family(&owner.token, "Guarded").await;
    let family_id = family["id"].as_str().unwrap().to_string();
    let valid = invite(&server, &owner.token, &family_id).await;

    for _ in 0..10 {
        let response = join(&server, &guesser.token, "not-a-code").await;
        assert_eq!(response.status(), 404);
    }
    let blocked = join(&server, &guesser.token, valid["code"].as_str().unwrap()).await;
    assert_eq!(blocked.status(), 429, "even a valid code is refused once the limit is reached");

    // The limit is per user, so someone else can still use the code.
    let other = server.register("other@example.com").await;
    assert_eq!(join(&server, &other.token, valid["code"].as_str().unwrap()).await.status(), 200);
}

#[tokio::test]
async fn preview_shows_the_family_without_using_the_code() {
    let Some(server) = spawn(true).await else { return };
    let owner = server.register("owner@example.com").await;
    let guest = server.register("guest@example.com").await;
    let outsider = server.register("outsider@example.com").await;
    let family = server.create_family(&owner.token, "The Morgans").await;
    let family_id = family["id"].as_str().unwrap().to_string();
    server.join_family(&owner.token, &family_id, &guest.token).await;
    let created = invite(&server, &owner.token, &family_id).await;
    let code = created["code"].as_str().unwrap().to_string();

    let preview = |token: &str, code: &str| {
        server.http.get(server.url(&format!("/invites/{code}"))).bearer_auth(token.to_string()).send()
    };
    let shown: Value = preview(&outsider.token, &code).await.unwrap().json().await.unwrap();
    assert_eq!(shown["family_name"], "The Morgans");
    assert_eq!(shown["invited_by"], "owner");
    assert_eq!(shown["member_count"], 2);
    assert_eq!(shown["member_names"], json!(["owner", "guest"]));

    let again = preview(&outsider.token, &code).await.unwrap();
    assert_eq!(again.status(), 200, "checking a code does not use it");

    assert_eq!(join(&server, &outsider.token, &code).await.status(), 200);
    let after_use = preview(&outsider.token, &code).await.unwrap();
    assert_eq!(after_use.status(), 404);
    assert_eq!(error_of(after_use).await, INVALID_CODE);
}

#[tokio::test]
async fn membership_changes_reach_the_affected_streams() {
    let Some(server) = spawn(true).await else { return };
    let owner = server.register("owner@example.com").await;
    let guest = server.register("guest@example.com").await;
    let family = server.create_family(&owner.token, "Live").await;
    let family_id = family["id"].as_str().unwrap().to_string();

    let mut owner_stream = server.open_stream(&owner.token).await;
    assert_eq!(owner_stream.next_event().await.kind, "op_ready", "the stream starts with the current seq");
    server.join_family(&owner.token, &family_id, &guest.token).await;
    assert_eq!(owner_stream.next_event().await.kind, "membership_changed", "the owner hears about the new member");

    let mut guest_stream = server.open_stream(&guest.token).await;
    assert_eq!(guest_stream.next_event().await.kind, "op_ready");
    let remove = server
        .http
        .delete(server.url(&format!("/family/{family_id}/members/{}", guest.user_id)))
        .bearer_auth(&owner.token)
        .send()
        .await
        .unwrap();
    assert_eq!(remove.status(), 204);
    assert_eq!(guest_stream.next_event().await.kind, "membership_changed", "the removed member is told");
}
