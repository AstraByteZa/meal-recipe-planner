//! Shared helpers for the integration tests. Each test gets its own database on the Postgres
//! server named by `TEST_DATABASE_URL`, and its own server on a random local port.

#![allow(dead_code)]

use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use chrono::Utc;
use futures_util::{Stream, StreamExt};
use serde_json::{Value, json};
use shared_core::time::format_timestamp;
use shared_core::wire::SseEvent;
use sqlx::PgPool;
use sync_server::hub::Hub;
use sync_server::{AppState, db, router};
use tokio::net::TcpListener;
use uuid::Uuid;

const SECRET: &str = "test-secret-that-is-long-enough-for-hs256";

pub struct TestServer {
    pub base: String,
    pub http: reqwest::Client,
}

pub struct Session {
    pub token: String,
    pub user_id: String,
}

/// Starts a server on a fresh database. Returns `None` when `TEST_DATABASE_URL` is unset, so
/// `cargo test` still runs without Postgres. Set it to run the server tests.
pub async fn spawn(registration_open: bool) -> Option<TestServer> {
    let Ok(admin_url) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("skipping sync-server integration test: set TEST_DATABASE_URL to run it");
        return None;
    };
    let name = format!("sync_test_{}", Uuid::new_v4().simple());
    let admin = PgPool::connect(&admin_url).await.expect("connect to TEST_DATABASE_URL");
    // The name is a fixed prefix plus a UUID, so there is no injection risk here.
    let create = format!("CREATE DATABASE {name}");
    sqlx::query(sqlx::AssertSqlSafe(create)).execute(&admin).await.expect("create test database");
    admin.close().await;

    let (server_url, _) = admin_url.rsplit_once('/').expect("TEST_DATABASE_URL has a database name");
    let pool = PgPool::connect(&format!("{server_url}/{name}")).await.expect("connect to test database");
    db::migrate(&pool).await.expect("migrate test database");

    let state = AppState {
        pool,
        jwt_secret: Arc::from(SECRET.as_bytes()),
        registration_open,
        hub: Hub::default(),
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind test port");
    let addr = listener.local_addr().expect("test port address");
    tokio::spawn(async move {
        axum::serve(listener, router(state)).await.expect("test server");
    });
    Some(TestServer { base: format!("http://{addr}"), http: reqwest::Client::new() })
}

/// A timestamp `seconds` before now, in the `updated_at` format. Relative to now so that the
/// server's clock-skew clamp never changes the ordering a test relies on.
pub fn at(seconds_before_now: i64) -> String {
    format_timestamp(Utc::now() - chrono::Duration::seconds(seconds_before_now))
}

pub fn new_id() -> String {
    Uuid::new_v4().to_string()
}

/// Builds an upsert op for a recipe-like row. `family` is the family id, or `None` for private.
pub fn upsert(entity: &str, id: &str, family: Option<&str>, timestamp: &str, name: &str) -> Value {
    json!({
        "entity_type": entity,
        "entity_id": id,
        "op": "UPSERT",
        "family_id": family,
        "client_timestamp": timestamp,
        "payload": { "id": id, "name": name },
    })
}

pub fn delete(entity: &str, id: &str, family: Option<&str>, timestamp: &str) -> Value {
    json!({
        "entity_type": entity,
        "entity_id": id,
        "op": "DELETE",
        "family_id": family,
        "client_timestamp": timestamp,
        "payload": null,
    })
}

impl TestServer {
    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    pub async fn register(&self, email: &str) -> Session {
        let response = self
            .http
            .post(self.url("/auth/register"))
            .json(&json!({ "email": email, "password": "correct horse battery" }))
            .send()
            .await
            .expect("register request");
        assert_eq!(response.status(), 201, "register {email}");
        let body: Value = response.json().await.expect("register body");
        Session {
            token: body["token"].as_str().expect("token").to_string(),
            user_id: body["user"]["id"].as_str().expect("user id").to_string(),
        }
    }

    pub async fn create_family(&self, token: &str, name: &str) -> Value {
        let response = self
            .http
            .post(self.url("/family/create"))
            .bearer_auth(token)
            .json(&json!({ "name": name }))
            .send()
            .await
            .expect("create family request");
        assert_eq!(response.status(), 201);
        response.json().await.expect("family body")
    }

    pub async fn push(&self, token: &str, client_id: &str, ops: Value) -> reqwest::Response {
        self.http
            .post(self.url("/sync/push"))
            .bearer_auth(token)
            .json(&json!({ "client_id": client_id, "ops": ops }))
            .send()
            .await
            .expect("push request")
    }

    /// Pushes ops that must all be accepted and returns the per-op statuses.
    pub async fn push_ok(&self, token: &str, client_id: &str, ops: Value) -> Vec<String> {
        let response = self.push(token, client_id, ops).await;
        assert_eq!(response.status(), 200, "push should succeed");
        let body: Value = response.json().await.expect("push body");
        body["results"]
            .as_array()
            .expect("results")
            .iter()
            .map(|result| result["status"].as_str().expect("status").to_string())
            .collect()
    }

    pub async fn pull(&self, token: &str, since: i64) -> Value {
        self.pull_with(token, since, None).await
    }

    pub async fn pull_with(&self, token: &str, since: i64, limit: Option<i64>) -> Value {
        let mut path = format!("/sync/pull?since={since}");
        if let Some(limit) = limit {
            path.push_str(&format!("&limit={limit}"));
        }
        let response = self.http.get(self.url(&path)).bearer_auth(token).send().await.expect("pull request");
        assert_eq!(response.status(), 200);
        response.json().await.expect("pull body")
    }

    pub async fn open_stream(&self, token: &str) -> EventStream {
        let response = self
            .http
            .get(self.url("/sync/stream"))
            .bearer_auth(token)
            .send()
            .await
            .expect("stream request");
        assert_eq!(response.status(), 200);
        assert_eq!(response.headers()["x-accel-buffering"], "no");
        EventStream {
            body: Box::pin(response.bytes_stream()),
            buffer: String::new(),
        }
    }
}

/// Reads `op_ready` events from a `/sync/stream` response.
pub struct EventStream {
    body: Pin<Box<dyn Stream<Item = reqwest::Result<Bytes>> + Send>>,
    buffer: String,
}

impl EventStream {
    /// Waits for the next event and returns its seq. Panics if none arrives within 5 seconds.
    pub async fn next_seq(&mut self) -> i64 {
        loop {
            if let Some(end) = self.buffer.find("\n\n") {
                let block: String = self.buffer.drain(..end + 2).collect();
                for line in block.lines() {
                    if let Some(data) = line.strip_prefix("data:") {
                        let event: SseEvent = serde_json::from_str(data.trim()).expect("event json");
                        assert_eq!(event.kind, SseEvent::OP_READY);
                        return event.seq;
                    }
                }
                continue;
            }
            let chunk = tokio::time::timeout(Duration::from_secs(5), self.body.next())
                .await
                .expect("timed out waiting for a stream event")
                .expect("stream ended")
                .expect("stream error");
            self.buffer.push_str(std::str::from_utf8(&chunk).expect("utf-8 event data"));
        }
    }
}
