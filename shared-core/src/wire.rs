//! Request and response bodies for the sync API, shared by the server and the client worker.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::entity::{EntityType, Operation};
use crate::time::parse_timestamp;

/// Upper bound on family and display names, in characters.
pub const MAX_NAME_CHARS: usize = 80;

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
    /// The log position for `op_ready`. `membership_changed` carries no seq, so it defaults to 0.
    #[serde(default)]
    pub seq: i64,
}

impl SseEvent {
    pub const OP_READY: &'static str = "op_ready";
    /// This user joined, left, was removed from, or lost a family, or the family was deleted.
    /// The client re-reads its families and restarts the stream.
    pub const MEMBERSHIP_CHANGED: &'static str = "membership_changed";

    pub fn op_ready(seq: i64) -> Self {
        Self { kind: Self::OP_READY.to_string(), seq }
    }

    pub fn membership_changed() -> Self {
        Self { kind: Self::MEMBERSHIP_CHANGED.to_string(), seq: 0 }
    }
}

/// A signed-in user's account details.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserDto {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
}

/// A user's role in one family. Each family has exactly one owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FamilyRole {
    Owner,
    Member,
}

/// A family as the caller sees it. `role` is the caller's own role in it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FamilyDto {
    pub id: Uuid,
    pub name: String,
    pub role: FamilyRole,
}

/// A family member. Every member can see each other's name and email.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberDto {
    pub user_id: Uuid,
    /// The display name, or the part of the email before `@` when no name is set.
    pub name: String,
    pub email: String,
    pub role: FamilyRole,
    /// In the same timestamp format as `updated_at`.
    pub joined_at: String,
}

/// A pending invite code. Only the owner receives these. `code` is shown as `XXXX-XXXX`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InviteDto {
    pub id: Uuid,
    pub code: String,
    pub created_at: String,
    pub expires_at: String,
}

/// Everything the Family page shows. `invites` is empty unless the caller is the owner.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FamilyDetails {
    pub family: FamilyDto,
    pub members: Vec<MemberDto>,
    pub invites: Vec<InviteDto>,
}

/// What an invite code leads to, shown before the user accepts. Checking a code does not use it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvitePreview {
    pub family_name: String,
    /// The name of the member who created the code.
    pub invited_by: String,
    pub member_count: i64,
    /// Up to three member names, for the avatars.
    pub member_names: Vec<String>,
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
    fn membership_event_parses_without_a_seq() {
        let event: SseEvent = serde_json::from_str(r#"{"type":"membership_changed"}"#).unwrap();
        assert_eq!(event, SseEvent::membership_changed());
        let ready: SseEvent = serde_json::from_str(r#"{"type":"op_ready","seq":7}"#).unwrap();
        assert_eq!(ready, SseEvent::op_ready(7));
    }

    #[test]
    fn family_role_is_snake_case_on_the_wire() {
        assert_eq!(serde_json::to_string(&FamilyRole::Owner).unwrap(), "\"owner\"");
        assert_eq!(serde_json::from_str::<FamilyRole>("\"member\"").unwrap(), FamilyRole::Member);
    }

    #[test]
    fn delete_does_not_need_a_matching_payload() {
        let mut op = sync_op(EntityType::Recipe, None, Value::Null);
        op.op = Operation::Delete;
        assert!(op.validate().is_ok());
    }
}
