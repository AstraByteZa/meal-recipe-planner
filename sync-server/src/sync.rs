//! Push, pull and the change stream. Each row is last-write-wins. Every accepted change is
//! appended to `op_log`, and clients pull that log from a `seq` cursor.

use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::time::Duration;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::{HeaderName, HeaderValue};
use axum::response::IntoResponse;
use axum::response::sse::{Event, KeepAlive, Sse};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use futures_util::future;
use futures_util::stream::{self, StreamExt, select_all};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use shared_core::lww::lww_wins;
use shared_core::time::format_timestamp;
use shared_core::wire::{
    MAX_OPS_PER_PUSH, OpResult, OpStatus, PullOp, PullResponse, PushRequest, PushResponse, SseEvent, SyncOp,
};
use sqlx::types::Json as SqlJson;
use sqlx::{FromRow, PgConnection};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult, internal};
use crate::family::family_ids;
use crate::hub::Notice;
use crate::state::AppState;

/// Transaction lock taken by every push. `seq` is assigned at insert time, but rows commit in
/// a different order, so a pull could skip a seq that commits late. Serialising pushes keeps
/// commit order equal to seq order. The key is just the ASCII bytes of "SYNC".
const PUSH_LOCK_KEY: i64 = 0x5359_4e43;
const MAX_CLIENT_ID_CHARS: usize = 64;
/// Client timestamps further ahead of the server clock than this are clamped. Otherwise a
/// clock set in the future would win every LWW comparison.
const MAX_CLOCK_SKEW_MINUTES: i64 = 5;
const DEFAULT_PULL_LIMIT: i64 = 500;
const MAX_PULL_LIMIT: i64 = 1000;
const KEEP_ALIVE_SECS: u64 = 15;

/// What happened to one op inside the push transaction.
enum Outcome {
    /// Stored and logged at `seq`. `channel` is the user (private) or family whose streams hear about it.
    Applied { seq: i64, channel: Uuid },
    Stale,
    Rejected(String),
}

pub async fn push(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Json(request): Json<PushRequest>,
) -> ApiResult<Json<PushResponse>> {
    let client_id = request.client_id.trim();
    if client_id.is_empty() || client_id.chars().count() > MAX_CLIENT_ID_CHARS {
        return Err(ApiError::BadRequest(format!("client_id must be 1 to {MAX_CLIENT_ID_CHARS} characters")));
    }
    if request.ops.len() > MAX_OPS_PER_PUSH {
        return Err(ApiError::BadRequest(format!("a push may carry at most {MAX_OPS_PER_PUSH} ops")));
    }

    let families: HashSet<Uuid> = family_ids(&state.pool, me).await?.into_iter().collect();
    let cap = Utc::now() + ChronoDuration::minutes(MAX_CLOCK_SKEW_MINUTES);

    let mut tx = state.pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)").bind(PUSH_LOCK_KEY).execute(&mut *tx).await?;

    let mut results = Vec::with_capacity(request.ops.len());
    let mut newest: HashMap<Uuid, i64> = HashMap::new();
    for op in &request.ops {
        let outcome = apply_op(&mut tx, me, client_id, op, &families, cap).await?;
        let (status, reason) = match outcome {
            Outcome::Applied { seq, channel } => {
                let latest = newest.entry(channel).or_insert(seq);
                *latest = (*latest).max(seq);
                (OpStatus::Applied, None)
            }
            Outcome::Stale => (OpStatus::Stale, None),
            Outcome::Rejected(reason) => (OpStatus::Rejected, Some(reason)),
        };
        results.push(OpResult { entity_id: op.entity_id, status, reason });
    }
    tx.commit().await?;

    // Only after commit, so a stream never announces a seq that a pull cannot yet see.
    for (channel, seq) in newest {
        state.hub.notify(channel, Notice::Ops(seq));
    }
    Ok(Json(PushResponse { results }))
}

async fn apply_op(
    conn: &mut PgConnection,
    me: Uuid,
    client_id: &str,
    op: &SyncOp,
    families: &HashSet<Uuid>,
    cap: DateTime<Utc>,
) -> ApiResult<Outcome> {
    let timestamp = match op.validate() {
        Ok(timestamp) => timestamp.min(cap),
        Err(reason) => return Ok(Outcome::Rejected(reason)),
    };
    if let Some(family) = op.family_id {
        if !families.contains(&family) {
            return Ok(Outcome::Rejected("you are not a member of that family".to_string()));
        }
    }

    // A row's scope is its family when shared, or its owner when private. That is the same
    // key `entities.scope_id` uses.
    let scope = op.family_id.unwrap_or(me);
    let entity_type = op.entity_type.as_str();
    let timestamp_text = format_timestamp(timestamp);
    let deleted_at = (op.op == shared_core::entity::Operation::Delete).then_some(timestamp);

    let current = sqlx::query_as::<_, (DateTime<Utc>, String)>(
        "SELECT updated_at, updated_by_client FROM entities \
         WHERE scope_id = $1 AND entity_type = $2 AND entity_id = $3 FOR UPDATE",
    )
    .bind(scope)
    .bind(entity_type)
    .bind(op.entity_id)
    .fetch_optional(&mut *conn)
    .await?;

    match current {
        Some((current_timestamp, current_client)) => {
            let current_text = format_timestamp(current_timestamp);
            if !lww_wins(&timestamp_text, client_id, &current_text, &current_client) {
                return Ok(Outcome::Stale);
            }
            sqlx::query(
                "UPDATE entities SET payload = $4, updated_at = $5, updated_by_client = $6, deleted_at = $7 \
                 WHERE scope_id = $1 AND entity_type = $2 AND entity_id = $3",
            )
            .bind(scope)
            .bind(entity_type)
            .bind(op.entity_id)
            .bind(SqlJson(&op.payload))
            .bind(timestamp)
            .bind(client_id)
            .bind(deleted_at)
            .execute(&mut *conn)
            .await?;
        }
        None => {
            sqlx::query(
                "INSERT INTO entities (entity_type, entity_id, owner_id, family_id, payload, updated_at, updated_by_client, deleted_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(entity_type)
            .bind(op.entity_id)
            .bind(me)
            .bind(op.family_id)
            .bind(SqlJson(&op.payload))
            .bind(timestamp)
            .bind(client_id)
            .bind(deleted_at)
            .execute(&mut *conn)
            .await?;
        }
    }

    let seq = sqlx::query_scalar::<_, i64>(
        "INSERT INTO op_log (entity_type, entity_id, op, owner_id, family_id, client_id, client_timestamp, payload) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING seq",
    )
    .bind(entity_type)
    .bind(op.entity_id)
    .bind(op.op.as_str())
    .bind(me)
    .bind(op.family_id)
    .bind(client_id)
    .bind(timestamp)
    .bind(SqlJson(&op.payload))
    .fetch_one(&mut *conn)
    .await?;

    Ok(Outcome::Applied { seq, channel: scope })
}

#[derive(Deserialize)]
pub struct PullQuery {
    #[serde(default)]
    pub since: i64,
    pub limit: Option<i64>,
}

#[derive(FromRow)]
struct LogRow {
    seq: i64,
    entity_type: String,
    entity_id: Uuid,
    op: String,
    family_id: Option<Uuid>,
    client_id: String,
    client_timestamp: DateTime<Utc>,
    payload: SqlJson<Value>,
}

pub async fn pull(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Query(query): Query<PullQuery>,
) -> ApiResult<Json<PullResponse>> {
    let limit = query.limit.unwrap_or(DEFAULT_PULL_LIMIT).clamp(1, MAX_PULL_LIMIT);
    let families = family_ids(&state.pool, me).await?;

    // Fetch one extra row to learn whether more are waiting.
    let mut rows = sqlx::query_as::<_, LogRow>(
        "SELECT seq, entity_type, entity_id, op, family_id, client_id, client_timestamp, payload FROM op_log \
         WHERE seq > $1 AND ((family_id IS NULL AND owner_id = $2) OR family_id = ANY($3)) \
         ORDER BY seq LIMIT $4",
    )
    .bind(query.since)
    .bind(me)
    .bind(&families)
    .bind(limit + 1)
    .fetch_all(&state.pool)
    .await?;

    let has_more = rows.len() as i64 > limit;
    rows.truncate(limit as usize);
    let ops = rows.into_iter().map(to_pull_op).collect::<ApiResult<Vec<_>>>()?;
    let last_seq = ops.last().map_or(query.since, |op| op.seq);
    Ok(Json(PullResponse { ops, last_seq, has_more }))
}

fn to_pull_op(row: LogRow) -> ApiResult<PullOp> {
    Ok(PullOp {
        seq: row.seq,
        entity_type: from_db(&row.entity_type)?,
        entity_id: row.entity_id,
        op: from_db(&row.op)?,
        family_id: row.family_id,
        client_id: row.client_id,
        client_timestamp: format_timestamp(row.client_timestamp),
        payload: row.payload.0,
    })
}

/// Reads a wire name stored as text (`track`, `UPSERT`) back into its enum.
fn from_db<T: DeserializeOwned>(text: &str) -> ApiResult<T> {
    serde_json::from_value(Value::String(text.to_string())).map_err(internal)
}

pub async fn stream(State(state): State<AppState>, AuthUser(me): AuthUser) -> ApiResult<impl IntoResponse> {
    let families = family_ids(&state.pool, me).await?;

    // Subscribe before reading the current seq. An op committed in between is then either
    // in the initial event or in the stream, and never in neither.
    let channels = std::iter::once(me).chain(families.iter().copied());
    let updates = select_all(channels.map(|channel| BroadcastStream::new(state.hub.subscribe(channel))));
    let initial = latest_seq(&state, me, families).await?;

    let events = updates
        .scan(initial, |latest, item| {
            let batch = match item {
                Ok(Notice::Ops(seq)) => {
                    *latest = seq;
                    vec![SseEvent::op_ready(seq)]
                }
                Ok(Notice::Membership) => vec![SseEvent::membership_changed()],
                // Notices were missed, and any of them may have been a membership change. Send
                // both, since the client re-reads and pulls anyway.
                Err(BroadcastStreamRecvError::Lagged(_)) => {
                    vec![SseEvent::op_ready(*latest), SseEvent::membership_changed()]
                }
            };
            future::ready(Some(batch))
        })
        .flat_map(stream::iter);
    let events = stream::once(future::ready(SseEvent::op_ready(initial)))
        .chain(events)
        .map(|event| Ok::<Event, Infallible>(sse_event(&event)));

    let sse = Sse::new(events).keep_alive(KeepAlive::new().interval(Duration::from_secs(KEEP_ALIVE_SECS)));
    // Some proxies buffer event streams unless told not to. nginx honours this header.
    Ok(([(HeaderName::from_static("x-accel-buffering"), HeaderValue::from_static("no"))], sse))
}

async fn latest_seq(state: &AppState, me: Uuid, families: Vec<Uuid>) -> ApiResult<i64> {
    let seq = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(MAX(seq), 0) FROM op_log \
         WHERE (family_id IS NULL AND owner_id = $1) OR family_id = ANY($2)",
    )
    .bind(me)
    .bind(families)
    .fetch_one(&state.pool)
    .await?;
    Ok(seq)
}

fn sse_event(event: &SseEvent) -> Event {
    Event::default().json_data(event).unwrap_or_else(|_| Event::default())
}
