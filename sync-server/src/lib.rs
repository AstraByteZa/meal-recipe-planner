//! Family sync server: accounts, families and the op log that clients push to and pull from.

pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod family;
pub mod hub;
pub mod state;
pub mod sync;

use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::Router;

pub use state::AppState;

/// Largest request body. A full sync batch of 500 recipes fits well inside this.
const MAX_BODY_BYTES: usize = 8 * 1024 * 1024;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/me", get(auth::me).patch(auth::update_me))
        .route("/family", get(family::list))
        .route("/family/create", post(family::create))
        .route("/family/join", post(family::join))
        .route("/sync/push", post(sync::push))
        .route("/sync/pull", get(sync::pull))
        .route("/sync/stream", get(sync::stream))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(state)
}
