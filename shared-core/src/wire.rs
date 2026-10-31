//! Request and response bodies for the sync API, shared by the server and the client worker.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::entity::{EntityType, Operation};
use crate::time::parse_timestamp;

/// Most ops the server accepts in one push. Clients split larger outboxes into batches.
pub const MAX_OPS_PER_PUSH: usize = 500;

/// One change as the client sends it. `payload` is the full row for an upsert. A delete may
/// carry the last known row or null.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SyncOp {
    pub entity_type: EntityType,
    pub entity_id: Uuid,
    pub op: Operation,
    /// The family that shares this row, or `None` for private data.
    pub family_id: Option<Uuid>,
    /// When the change was made, in the `updated_at` format. This is the LWW version.
    pub client_timestamp: String,
    pub payload: Value,
}

impl SyncOp {
    /// Checks the rules the server enforces. Returns the parsed client timestamp.
    pub fn validate(&self) -> Result<DateTime<Utc>, String> {
        if self.family_id.is_some() && !self.entity_type.is_family_shareable() {
            return Err(format!("{} rows cannot be shared with a family", self.entity_type.as_str()));
        }
        let timestamp = parse_timestamp(&self.client_timestamp)?;
        if self.op == Operation::Upsert {
            let id = self.payload.as_object().and_then(|row| row.get("id")).and_then(Value::as_str);
            if id.and_then(|id| Uuid::parse_str(id).ok()) != Some(self.entity_id) {
                return Err("upsert payload id must match entity_id".to_string());
            }
        }
        Ok(timestamp)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PushRequest {
    /// The device that made the changes. It breaks ties between equal timestamps.
    pub client_id: String,
    pub ops: Vec<SyncOp>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpStatus {
    /// The op won LWW and is now stored and logged.
    Applied,
    /// A newer version already exists. Nothing changed.
    Stale,
    /// The op broke a rule. Retrying it will not help, so the client drops it.
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OpResult {
    pub entity_id: Uuid,
    pub status: OpStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// One result per op, in the same order as the request.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PushResponse {
    pub results: Vec<OpResult>,
}

/// An op from the server log, as the client applies it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PullOp {
    pub seq: i64,
    pub entity_type: EntityType,
    pub entity_id: Uuid,
    pub op: Operation,
    pub family_id: Option<Uuid>,
    pub client_id: String,
    pub client_timestamp: String,
    pub payload: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PullResponse {
    pub ops: Vec<PullOp>,
    /// The cursor to send as `since` next time: the last returned seq, or `since` if none.
    pub last_seq: i64,
    /// True when more ops are waiting. Pull again from `last_seq`.
    pub has_more: bool,
}

/// The data of a `/sync/stream` event. It only says that something changed. The client pulls
/// to get the details, so a dropped or lagged event costs nothing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SseEvent {
    #[serde(rename = "type")]
    pub kind: String,
    pub seq: i64,
}

impl SseEvent {
    pub const OP_READY: &'static str = "op_ready";

    pub fn op_ready(seq: i64) -> Self {
        Self { kind: Self::OP_READY.to_string(), seq }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const ID: &str = "11111111-1111-4111-8111-111111111111";
    const TS: &str = "2026-10-31T10:00:00.000Z";

    fn sync_op(entity_type: EntityType, family_id: Option<Uuid>, payload: Value) -> SyncOp {
        SyncOp {
            entity_type,
            entity_id: Uuid::parse_str(ID).unwrap(),
            op: Operation::Upsert,
            family_id,
            client_timestamp: TS.to_string(),
            payload,
        }
    }

    #[test]
    fn valid_upsert_passes() {
        let op = sync_op(EntityType::Recipe, None, json!({"id": ID, "name": "Soup"}));
        assert!(op.validate().is_ok());
    }

    #[test]
    fn payload_id_must_match_entity_id() {
        let other = "22222222-2222-4222-8222-222222222222";
        let op = sync_op(EntityType::Recipe, None, json!({"id": other}));
        assert!(op.validate().is_err());
        let missing = sync_op(EntityType::Recipe, None, json!({"name": "Soup"}));
        assert!(missing.validate().is_err());
    }

    #[test]
    fn private_types_cannot_carry_a_family() {
        let family = Some(Uuid::nil());
        for entity in [EntityType::Settings, EntityType::HouseholdDiet, EntityType::Config] {
            assert!(sync_op(entity, family, json!({"id": ID})).validate().is_err());
        }
        assert!(sync_op(EntityType::Recipe, family, json!({"id": ID})).validate().is_ok());
    }

    #[test]
    fn bad_timestamp_is_rejected() {
        let mut op = sync_op(EntityType::Recipe, None, json!({"id": ID}));
        op.client_timestamp = "not a time".to_string();
        assert!(op.validate().is_err());
    }

    #[test]
    fn delete_does_not_need_a_matching_payload() {
        let mut op = sync_op(EntityType::Recipe, None, Value::Null);
        op.op = Operation::Delete;
        assert!(op.validate().is_ok());
    }
}
