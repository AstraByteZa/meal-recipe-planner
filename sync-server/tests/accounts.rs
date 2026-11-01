//! Accounts, families and the server's basic HTTP behaviour.

mod common;

use common::spawn;
use serde_json::{Value, json};

#[tokio::test]
async fn register_login_and_me() {
    let Some(server) = spawn(true).await else { return };
    let session = server.register("Cook@Example.com").await;

    let login = server
        .http
        .post(server.url("/auth/login"))
        .json(&json!({ "email": "cook@example.com", "password": "correct horse battery" }))
        .send()
        .await
        .unwrap();
    assert_eq!(login.status(), 200, "login is case-insensitive on email");

    let wrong = server
        .http
        .post(server.url("/auth/login"))
        .json(&json!({ "email": "cook@example.com", "password": "not the password" }))
        .send()
        .await
        .unwrap();
    assert_eq!(wrong.status(), 401);

    let anonymous = server.http.get(server.url("/me")).send().await.unwrap();
    assert_eq!(anonymous.status(), 401);

    let me: Value = server
        .http
        .get(server.url("/me"))
        .bearer_auth(&session.token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(me["email"], "cook@example.com");
    assert_eq!(me["id"], session.user_id);

    let renamed = server
        .http
        .patch(server.url("/me"))
        .bearer_auth(&session.token)
        .json(&json!({ "display_name": "Roelof" }))
        .send()
        .await
        .unwrap();
    assert_eq!(renamed.status(), 200);
    let renamed: Value = renamed.json().await.unwrap();
    assert_eq!(renamed["display_name"], "Roelof");
}

#[tokio::test]
async fn duplicate_email_is_a_conflict() {
    let Some(server) = spawn(true).await else { return };
    server.register("dup@example.com").await;
    let again = server
        .http
        .post(server.url("/auth/register"))
        .json(&json!({ "email": "DUP@example.com", "password": "correct horse battery" }))
        .send()
        .await
        .unwrap();
    assert_eq!(again.status(), 409);
}

#[tokio::test]
async fn short_passwords_are_rejected() {
    let Some(server) = spawn(true).await else { return };
    let response = server
        .http
        .post(server.url("/auth/register"))
        .json(&json!({ "email": "short@example.com", "password": "short" }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn closed_registration_refuses_new_accounts() {
    let Some(server) = spawn(false).await else { return };
    let response = server
        .http
        .post(server.url("/auth/register"))
        .json(&json!({ "email": "new@example.com", "password": "correct horse battery" }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 403);
}
