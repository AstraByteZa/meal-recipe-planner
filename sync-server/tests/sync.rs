//! Push, pull and the stream: last-write-wins, op validation, scoping and notifications.

mod common;

use common::{at, delete, new_id, spawn, upsert};
use serde_json::{Value, json};

#[tokio::test]
async fn newer_write_wins_and_stale_write_is_rejected() {
    let Some(server) = spawn(true).await else { return };
    let cook = server.register("lww@example.com").await;
    let id = new_id();

    let first = server.push_ok(&cook.token, "phone", json!([upsert("recipe", &id, None, &at(300), "Soup")])).await;
    assert_eq!(first, ["applied"]);

    let newer = server.push_ok(&cook.token, "phone", json!([upsert("recipe", &id, None, &at(200), "Better soup")])).await;
    assert_eq!(newer, ["applied"]);

    let stale = server.push_ok(&cook.token, "laptop", json!([upsert("recipe", &id, None, &at(250), "Old soup")])).await;
    assert_eq!(stale, ["stale"]);

    // Only the accepted writes reach the log, so a stale write is never pulled.
    let pulled = server.pull(&cook.token, 0).await;
    let names: Vec<&str> = pulled["ops"].as_array().unwrap().iter().map(|op| op["payload"]["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["Soup", "Better soup"]);
}

#[tokio::test]
async fn equal_timestamps_go_to_the_larger_client_id() {
    let Some(server) = spawn(true).await else { return };
    let cook = server.register("tie@example.com").await;
    let id = new_id();
    let same = at(100);

    assert_eq!(server.push_ok(&cook.token, "a-device", json!([upsert("tag", &id, None, &same, "one")])).await, ["applied"]);
    assert_eq!(server.push_ok(&cook.token, "z-device", json!([upsert("tag", &id, None, &same, "two")])).await, ["applied"]);
    assert_eq!(server.push_ok(&cook.token, "a-device", json!([upsert("tag", &id, None, &same, "three")])).await, ["stale"]);
}

#[tokio::test]
async fn replaying_a_push_changes_nothing() {
    let Some(server) = spawn(true).await else { return };
    let cook = server.register("replay@example.com").await;
    let op = upsert("recipe", &new_id(), None, &at(100), "Pie");

    assert_eq!(server.push_ok(&cook.token, "phone", json!([op.clone()])).await, ["applied"]);
    assert_eq!(server.push_ok(&cook.token, "phone", json!([op])).await, ["stale"]);
    assert_eq!(server.pull(&cook.token, 0).await["ops"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn delete_is_logged_and_pulled() {
    let Some(server) = spawn(true).await else { return };
    let cook = server.register("delete@example.com").await;
    let id = new_id();
    server.push_ok(&cook.token, "phone", json!([upsert("recipe", &id, None, &at(200), "Gone")])).await;
    assert_eq!(server.push_ok(&cook.token, "phone", json!([delete("recipe", &id, None, &at(100))])).await, ["applied"]);

    let pulled = server.pull(&cook.token, 0).await;
    let ops = pulled["ops"].as_array().unwrap();
    assert_eq!(ops[1]["op"], "DELETE");
    assert_eq!(ops[1]["entity_id"], id);
}

#[tokio::test]
async fn invalid_ops_are_rejected_without_failing_the_batch() {
    let Some(server) = spawn(true).await else { return };
    let cook = server.register("invalid@example.com").await;
    let family = server.create_family(&cook.token, "Home").await;
    let family_id = family["id"].as_str().unwrap();
    let stranger_family = new_id();

    let response = server
        .push(
            &cook.token,
            "phone",
            json!([
                upsert("recipe", &new_id(), None, &at(100), "Fine"),
                upsert("settings", &new_id(), Some(family_id), &at(100), "Private"),
                upsert("recipe", &new_id(), Some(&stranger_family), &at(100), "Not mine"),
                json!({
                    "entity_type": "recipe",
                    "entity_id": new_id(),
                    "op": "UPSERT",
                    "family_id": null,
                    "client_timestamp": "not a time",
                    "payload": {},
                }),
            ]),
        )
        .await;
    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    let results = body["results"].as_array().unwrap();
    let outcome: Vec<&str> = results.iter().map(|result| result["status"].as_str().unwrap()).collect();
    assert_eq!(outcome, ["applied", "rejected", "rejected", "rejected"]);
    assert!(results[1]["reason"].as_str().unwrap().contains("cannot be shared"));
    assert!(results[2]["reason"].as_str().unwrap().contains("not a member"));
}

#[tokio::test]
async fn payload_id_must_match_the_entity() {
    let Some(server) = spawn(true).await else { return };
    let cook = server.register("mismatch@example.com").await;
    let other = new_id();
    let op = json!({
        "entity_type": "recipe",
        "entity_id": new_id(),
        "op": "UPSERT",
        "family_id": null,
        "client_timestamp": at(100),
        "payload": { "id": other, "name": "Wrong row" },
    });
    assert_eq!(server.push_ok(&cook.token, "phone", json!([op])).await, ["rejected"]);
}

#[tokio::test]
async fn pull_hides_private_rows_from_others_and_shares_family_rows() {
    let Some(server) = spawn(true).await else { return };
    let alice = server.register("alice@example.com").await;
    let bob = server.register("bob@example.com").await;
    let family = server.create_family(&alice.token, "Kitchen").await;
    let family_id = family["id"].as_str().unwrap();
    let invite = family["invite_code"].as_str().unwrap();

    let private_id = new_id();
    let shared_id = new_id();
    server
        .push_ok(
            &alice.token,
            "phone",
            json!([
                upsert("recipe", &private_id, None, &at(200), "Mine"),
                upsert("recipe", &shared_id, Some(family_id), &at(100), "Ours"),
            ]),
        )
        .await;

    // Bob is not in the family yet, so he sees nothing.
    assert_eq!(server.pull(&bob.token, 0).await["ops"], json!([]));

    let alice_names: Vec<String> = server.pull(&alice.token, 0).await["ops"]
        .as_array()
        .unwrap()
        .iter()
        .map(|op| op["payload"]["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(alice_names, ["Mine", "Ours"]);

    // After joining, Bob sees the family row but still not Alice's private one.
    server
        .http
        .post(server.url("/family/join"))
        .bearer_auth(&bob.token)
        .json(&json!({ "invite_code": invite }))
        .send()
        .await
        .unwrap();
    let bob_ops = server.pull(&bob.token, 0).await["ops"].clone();
    let bob_names: Vec<&str> = bob_ops.as_array().unwrap().iter().map(|op| op["payload"]["name"].as_str().unwrap()).collect();
    assert_eq!(bob_names, ["Ours"]);
}

#[tokio::test]
async fn pull_pages_with_a_cursor() {
    let Some(server) = spawn(true).await else { return };
    let cook = server.register("pages@example.com").await;
    let ops: Vec<Value> = (0..3).map(|i| upsert("tag", &new_id(), None, &at(100 - i), &format!("t{i}"))).collect();
    server.push_ok(&cook.token, "phone", json!(ops)).await;

    let first = server.pull_with(&cook.token, 0, Some(2)).await;
    assert_eq!(first["ops"].as_array().unwrap().len(), 2);
    assert_eq!(first["has_more"], true);

    let second = server.pull_with(&cook.token, first["last_seq"].as_i64().unwrap(), Some(2)).await;
    assert_eq!(second["ops"].as_array().unwrap().len(), 1);
    assert_eq!(second["has_more"], false);

    let caught_up = server.pull(&cook.token, second["last_seq"].as_i64().unwrap()).await;
    assert_eq!(caught_up["ops"], json!([]));
    assert_eq!(caught_up["last_seq"], second["last_seq"]);
}

#[tokio::test]
async fn stream_announces_family_changes_but_not_private_ones() {
    let Some(server) = spawn(true).await else { return };
    let alice = server.register("stream-a@example.com").await;
    let bob = server.register("stream-b@example.com").await;
    let family = server.create_family(&alice.token, "Stream").await;
    let family_id = family["id"].as_str().unwrap().to_string();
    server
        .http
        .post(server.url("/family/join"))
        .bearer_auth(&bob.token)
        .json(&json!({ "invite_code": family["invite_code"] }))
        .send()
        .await
        .unwrap();

    let mut stream = server.open_stream(&bob.token).await;
    let initial = stream.next_seq().await;
    assert_eq!(initial, 0, "nothing has been pushed yet");

    // A private push must not wake Bob. The next event he gets should be the family push.
    server.push_ok(&alice.token, "phone", json!([upsert("recipe", &new_id(), None, &at(200), "Private")])).await;
    let shared_id = new_id();
    server
        .push_ok(&alice.token, "phone", json!([upsert("recipe", &shared_id, Some(&family_id), &at(100), "Shared")]))
        .await;

    let announced = stream.next_seq().await;
    let pulled = server.pull(&alice.token, 0).await;
    let shared_seq = pulled["ops"]
        .as_array()
        .unwrap()
        .iter()
        .find(|op| op["entity_id"] == shared_id)
        .unwrap()["seq"]
        .as_i64()
        .unwrap();
    assert_eq!(announced, shared_seq);
}

#[tokio::test]
async fn stream_sends_the_current_seq_on_connect() {
    let Some(server) = spawn(true).await else { return };
    let cook = server.register("connect@example.com").await;
    server.push_ok(&cook.token, "phone", json!([upsert("tag", &new_id(), None, &at(100), "t")])).await;
    let pulled = server.pull(&cook.token, 0).await;
    let last = pulled["last_seq"].as_i64().unwrap();

    let mut stream = server.open_stream(&cook.token).await;
    assert_eq!(stream.next_seq().await, last);
}
