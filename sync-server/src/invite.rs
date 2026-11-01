//! Invite codes. The owner creates single-use codes that expire after a week. A code is shown as
//! `XXXX-XXXX` and matched without regard to case, spaces or dashes. Checking a code does not use
//! it. Only accepting does.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use chrono::{DateTime, Duration, Utc};
use rand_core::{OsRng, RngCore};
use serde::Deserialize;
use shared_core::time::format_timestamp;
use shared_core::wire::{FamilyDto, FamilyRole, InviteDto, InvitePreview};
use sqlx::{FromRow, PgConnection, PgPool};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::family::{member_ids, member_name, member_role, notify_membership, require_owner, role_to_db};
use crate::state::AppState;

const INVITE_DAYS: i64 = 7;
const MAX_PENDING_INVITES: usize = 10;
const CODE_LENGTH: usize = 8;
/// No look-alikes: no I, L, O, 0 or 1. There are 31 characters.
const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
/// The largest multiple of 31 below 256. Bytes at or above it are redrawn, so every character
/// is equally likely.
const UNBIASED_BYTE_LIMIT: u8 = 248;

#[derive(Deserialize)]
pub struct JoinFamilyRequest {
    pub invite_code: String,
}

#[derive(FromRow)]
struct InviteRow {
    id: Uuid,
    code: String,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

/// The codes that can still be accepted. Only the owner reads these.
pub async fn pending(conn: &mut PgConnection, family: Uuid, now: DateTime<Utc>) -> ApiResult<Vec<InviteDto>> {
    let rows = sqlx::query_as::<_, InviteRow>(
        "SELECT id, code, created_at, expires_at FROM family_invites \
         WHERE family_id = $1 AND used_at IS NULL AND revoked_at IS NULL AND expires_at > $2 \
         ORDER BY created_at",
    )
    .bind(family)
    .bind(now)
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows.into_iter().map(invite_dto).collect())
}

pub async fn create(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Path(family): Path<Uuid>,
) -> ApiResult<(StatusCode, Json<InviteDto>)> {
    let mut tx = state.pool.begin().await?;
    require_owner(&mut tx, family, me).await?;
    // Lock the family row so two concurrent creates cannot both pass the cap check.
    sqlx::query("SELECT id FROM families WHERE id = $1 FOR UPDATE")
        .bind(family)
        .execute(&mut *tx)
        .await?;

    let now = Utc::now();
    if pending(&mut tx, family, now).await?.len() >= MAX_PENDING_INVITES {
        return Err(ApiError::Conflict(format!(
            "this family already has {MAX_PENDING_INVITES} pending invites. Revoke one before making another"
        )));
    }

    let id = Uuid::new_v4();
    let code = new_code();
    let expires_at = now + Duration::days(INVITE_DAYS);
    sqlx::query(
        "INSERT INTO family_invites (id, family_id, code, created_by, created_at, expires_at) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(family)
    .bind(&code)
    .bind(me)
    .bind(now)
    .bind(expires_at)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let invite = InviteDto {
        id,
        code: display_code(&code),
        created_at: format_timestamp(now),
        expires_at: format_timestamp(expires_at),
    };
    Ok((StatusCode::CREATED, Json(invite)))
}

pub async fn revoke(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Path((family, invite)): Path<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    let mut tx = state.pool.begin().await?;
    require_owner(&mut tx, family, me).await?;
    let revoked = sqlx::query(
        "UPDATE family_invites SET revoked_at = $3 \
         WHERE id = $1 AND family_id = $2 AND used_at IS NULL AND revoked_at IS NULL",
    )
    .bind(invite)
    .bind(family)
    .bind(Utc::now())
    .execute(&mut *tx)
    .await?;
    if revoked.rows_affected() == 0 {
        return Err(ApiError::NotFound("that invite is not pending".to_string()));
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Accepts a code. Members who use a code keep it unused. Every other failure gives the same
/// message, so a caller cannot tell a revoked code from a mistyped one.
pub async fn join(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Json(request): Json<JoinFamilyRequest>,
) -> ApiResult<Json<FamilyDto>> {
    state.join_throttle.check(me)?;
    match accept(&state.pool, me, &request.invite_code).await? {
        Some((family, members)) => {
            notify_membership(&state, &members);
            Ok(Json(family))
        }
        None => {
            state.join_throttle.record_failure(me);
            Err(invalid_code())
        }
    }
}

/// What a code leads to, without using it. Shown before the user accepts.
pub async fn preview(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Path(code): Path<String>,
) -> ApiResult<Json<InvitePreview>> {
    state.join_throttle.check(me)?;
    let found = match normalize(&code) {
        Some(code) => load_preview(&state.pool, &code, Utc::now()).await?,
        None => None,
    };
    match found {
        Some(preview) => Ok(Json(preview)),
        None => {
            state.join_throttle.record_failure(me);
            Err(invalid_code())
        }
    }
}

/// Uses the code and adds `me` as a member, in one transaction. Returns the family and the
/// members to notify, or `None` when the code is not usable. An existing member gets the family
/// back and the code stays unused.
async fn accept(pool: &PgPool, me: Uuid, raw: &str) -> ApiResult<Option<(FamilyDto, Vec<Uuid>)>> {
    let Some(code) = normalize(raw) else {
        return Ok(None);
    };
    let now = Utc::now();
    let mut tx = pool.begin().await?;
    // Locking the row serialises two people racing for the same code.
    let Some((invite, family)) = sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT id, family_id FROM family_invites \
         WHERE code = $1 AND used_at IS NULL AND revoked_at IS NULL AND expires_at > $2 FOR UPDATE",
    )
    .bind(&code)
    .bind(now)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(None);
    };

    let name: String = sqlx::query_scalar("SELECT name FROM families WHERE id = $1")
        .bind(family)
        .fetch_one(&mut *tx)
        .await?;
    if let Some(role) = member_role(&mut tx, family, me).await? {
        tx.commit().await?;
        return Ok(Some((FamilyDto { id: family, name, role }, Vec::new())));
    }

    sqlx::query("UPDATE family_invites SET used_by = $2, used_at = $3 WHERE id = $1")
        .bind(invite)
        .bind(me)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO family_members (family_id, user_id, role) VALUES ($1, $2, $3)")
        .bind(family)
        .bind(me)
        .bind(role_to_db(FamilyRole::Member))
        .execute(&mut *tx)
        .await?;
    let members = member_ids(&mut tx, family).await?;
    tx.commit().await?;
    Ok(Some((FamilyDto { id: family, name, role: FamilyRole::Member }, members)))
}

async fn load_preview(pool: &PgPool, code: &str, now: DateTime<Utc>) -> ApiResult<Option<InvitePreview>> {
    let Some((family, family_name, inviter_name, inviter_email)) = sqlx::query_as::<_, (Uuid, String, String, String)>(
        "SELECT i.family_id, f.name, u.display_name, u.email FROM family_invites i \
         JOIN families f ON f.id = i.family_id JOIN users u ON u.id = i.created_by \
         WHERE i.code = $1 AND i.used_at IS NULL AND i.revoked_at IS NULL AND i.expires_at > $2",
    )
    .bind(code)
    .bind(now)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };

    let member_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM family_members WHERE family_id = $1")
        .bind(family)
        .fetch_one(pool)
        .await?;
    let first_members = sqlx::query_as::<_, (String, String)>(
        "SELECT u.display_name, u.email FROM family_members m JOIN users u ON u.id = m.user_id \
         WHERE m.family_id = $1 ORDER BY m.joined_at, u.email LIMIT 3",
    )
    .bind(family)
    .fetch_all(pool)
    .await?;

    Ok(Some(InvitePreview {
        family_name,
        invited_by: member_name(&inviter_name, &inviter_email),
        member_count,
        member_names: first_members.iter().map(|(name, email)| member_name(name, email)).collect(),
    }))
}

fn invite_dto(row: InviteRow) -> InviteDto {
    InviteDto {
        id: row.id,
        code: display_code(&row.code),
        created_at: format_timestamp(row.created_at),
        expires_at: format_timestamp(row.expires_at),
    }
}

/// The one message for every unusable code. Revoked, expired, used and mistyped codes all look the same.
fn invalid_code() -> ApiError {
    ApiError::NotFound("That invite code is not valid or has expired".to_string())
}

/// The stored form: 8 uppercase characters, no dash. `None` when the input cannot be a code.
pub fn normalize(input: &str) -> Option<String> {
    let code: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_ascii_uppercase())
        .collect();
    let valid = code.len() == CODE_LENGTH && code.bytes().all(|byte| CODE_ALPHABET.contains(&byte));
    valid.then_some(code)
}

/// The shown form, `XXXX-XXXX`. Expects the stored form.
pub fn display_code(code: &str) -> String {
    format!("{}-{}", &code[..CODE_LENGTH / 2], &code[CODE_LENGTH / 2..])
}

fn new_code() -> String {
    let mut code = String::with_capacity(CODE_LENGTH);
    let mut byte = [0u8; 1];
    while code.len() < CODE_LENGTH {
        OsRng.fill_bytes(&mut byte);
        if byte[0] < UNBIASED_BYTE_LIMIT {
            code.push(CODE_ALPHABET[byte[0] as usize % CODE_ALPHABET.len()] as char);
        }
    }
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_normalize_regardless_of_case_dashes_and_spaces() {
        assert_eq!(normalize(" k7m2-qx9p ").as_deref(), Some("K7M2QX9P"));
        assert_eq!(normalize("K7M2QX9P").as_deref(), Some("K7M2QX9P"));
    }

    #[test]
    fn look_alike_and_wrong_length_codes_are_rejected() {
        assert_eq!(normalize("ILO0-1111"), None);
        assert_eq!(normalize("K7M2-QX9"), None);
        assert_eq!(normalize("K7M2-QX9PP"), None);
        assert_eq!(normalize(""), None);
    }

    #[test]
    fn generated_codes_are_valid_and_display_as_four_four() {
        for _ in 0..200 {
            let code = new_code();
            assert_eq!(normalize(&code).as_deref(), Some(code.as_str()));
            assert_eq!(display_code(&code).len(), 9);
            assert_eq!(&display_code(&code)[4..5], "-");
        }
    }
}
