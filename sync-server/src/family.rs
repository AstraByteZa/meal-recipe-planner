//! Families: a group of users who share rows. Each family has one owner. Members join with
//! single-use invite codes (`invite.rs`) and are removed, leave or change owner in `member.rs`.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use shared_core::time::format_timestamp;
use shared_core::wire::{FamilyDetails, FamilyDto, FamilyRole, MAX_NAME_CHARS, MemberDto};
use sqlx::{FromRow, PgConnection, PgPool};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::invite;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateFamilyRequest {
    pub name: String,
}

#[derive(Deserialize)]
pub struct RenameFamilyRequest {
    pub name: String,
}

#[derive(FromRow)]
struct FamilyRow {
    id: Uuid,
    name: String,
    role: String,
}

#[derive(FromRow)]
struct MemberRow {
    user_id: Uuid,
    display_name: String,
    email: String,
    role: String,
    joined_at: DateTime<Utc>,
}

/// The families a user belongs to. Sync uses this to decide which shared rows they can see.
pub async fn family_ids(pool: &PgPool, user: Uuid) -> ApiResult<Vec<Uuid>> {
    let ids = sqlx::query_scalar("SELECT family_id FROM family_members WHERE user_id = $1")
        .bind(user)
        .fetch_all(pool)
        .await?;
    Ok(ids)
}

pub async fn list(State(state): State<AppState>, AuthUser(me): AuthUser) -> ApiResult<Json<Vec<FamilyDto>>> {
    let rows = sqlx::query_as::<_, FamilyRow>(
        "SELECT f.id, f.name, m.role FROM families f \
         JOIN family_members m ON m.family_id = f.id WHERE m.user_id = $1 ORDER BY f.name",
    )
    .bind(me)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows.into_iter().map(family_dto).collect::<ApiResult<Vec<_>>>()?))
}

pub async fn create(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Json(request): Json<CreateFamilyRequest>,
) -> ApiResult<(StatusCode, Json<FamilyDto>)> {
    let name = validate_name(&request.name)?;
    let id = Uuid::new_v4();

    let mut tx = state.pool.begin().await?;
    sqlx::query("INSERT INTO families (id, name) VALUES ($1, $2)")
        .bind(id)
        .bind(&name)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO family_members (family_id, user_id, role) VALUES ($1, $2, 'owner')")
        .bind(id)
        .bind(me)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(FamilyDto { id, name, role: FamilyRole::Owner })))
}

/// The family and everything its members can see. Non-members get 404, so ids do not leak.
pub async fn details(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<FamilyDetails>> {
    let mut tx = state.pool.begin().await?;
    let role = require_member(&mut tx, id, me).await?;
    let name: String = sqlx::query_scalar("SELECT name FROM families WHERE id = $1")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    let member_rows = sqlx::query_as::<_, MemberRow>(
        "SELECT m.user_id, u.display_name, u.email, m.role, m.joined_at FROM family_members m \
         JOIN users u ON u.id = m.user_id WHERE m.family_id = $1 \
         ORDER BY (m.role = 'owner') DESC, m.joined_at, u.email",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;
    let invites = if role == FamilyRole::Owner {
        invite::pending(&mut tx, id, Utc::now()).await?
    } else {
        Vec::new()
    };
    tx.commit().await?;

    let members = member_rows.into_iter().map(member_dto).collect::<ApiResult<Vec<_>>>()?;
    Ok(Json(FamilyDetails { family: FamilyDto { id, name, role }, members, invites }))
}

pub async fn rename(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Path(id): Path<Uuid>,
    Json(request): Json<RenameFamilyRequest>,
) -> ApiResult<Json<FamilyDto>> {
    let name = validate_name(&request.name)?;
    let mut tx = state.pool.begin().await?;
    require_owner(&mut tx, id, me).await?;
    sqlx::query("UPDATE families SET name = $2 WHERE id = $1")
        .bind(id)
        .bind(&name)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(FamilyDto { id, name, role: FamilyRole::Owner }))
}

/// Deletes the family and, through foreign keys, its shared rows, invites and members. The
/// members keep their private rows. Each member is told, so their stream re-reads families.
pub async fn delete(State(state): State<AppState>, AuthUser(me): AuthUser, Path(id): Path<Uuid>) -> ApiResult<StatusCode> {
    let mut tx = state.pool.begin().await?;
    require_owner(&mut tx, id, me).await?;
    let members = member_ids(&mut tx, id).await?;
    sqlx::query("DELETE FROM families WHERE id = $1").bind(id).execute(&mut *tx).await?;
    tx.commit().await?;
    notify_membership(&state, &members);
    Ok(StatusCode::NO_CONTENT)
}

/// The caller's role in `family`, or 404 when they are not a member.
pub async fn require_member(conn: &mut PgConnection, family: Uuid, user: Uuid) -> ApiResult<FamilyRole> {
    member_role(conn, family, user)
        .await?
        .ok_or_else(|| ApiError::NotFound("family not found".to_string()))
}

/// Like `require_member`, but also requires the owner role (403 for other members).
pub async fn require_owner(conn: &mut PgConnection, family: Uuid, user: Uuid) -> ApiResult<()> {
    match require_member(conn, family, user).await? {
        FamilyRole::Owner => Ok(()),
        FamilyRole::Member => Err(ApiError::Forbidden("only the family owner can do that".to_string())),
    }
}

pub async fn member_role(conn: &mut PgConnection, family: Uuid, user: Uuid) -> ApiResult<Option<FamilyRole>> {
    let role: Option<String> =
        sqlx::query_scalar("SELECT role FROM family_members WHERE family_id = $1 AND user_id = $2")
            .bind(family)
            .bind(user)
            .fetch_optional(&mut *conn)
            .await?;
    role.map(|role| role_from_db(&role)).transpose()
}

/// Every current member of `family`. Read before a change so the people it removes are included.
pub async fn member_ids(conn: &mut PgConnection, family: Uuid) -> ApiResult<Vec<Uuid>> {
    Ok(sqlx::query_scalar("SELECT user_id FROM family_members WHERE family_id = $1")
        .bind(family)
        .fetch_all(&mut *conn)
        .await?)
}

/// Tells each user's streams that their membership changed. Call only after the commit.
pub fn notify_membership(state: &AppState, users: &[Uuid]) {
    for user in users {
        state.hub.notify(*user, crate::hub::Notice::Membership);
    }
}

pub fn role_to_db(role: FamilyRole) -> &'static str {
    match role {
        FamilyRole::Owner => "owner",
        FamilyRole::Member => "member",
    }
}

fn role_from_db(text: &str) -> ApiResult<FamilyRole> {
    match text {
        "owner" => Ok(FamilyRole::Owner),
        "member" => Ok(FamilyRole::Member),
        other => Err(crate::error::internal(format!("unknown family role {other}"))),
    }
}

fn family_dto(row: FamilyRow) -> ApiResult<FamilyDto> {
    Ok(FamilyDto { id: row.id, name: row.name, role: role_from_db(&row.role)? })
}

fn member_dto(row: MemberRow) -> ApiResult<MemberDto> {
    Ok(MemberDto {
        user_id: row.user_id,
        name: member_name(&row.display_name, &row.email),
        role: role_from_db(&row.role)?,
        email: row.email,
        joined_at: format_timestamp(row.joined_at),
    })
}

/// The display name, or the part of the email before `@` when none is set.
pub fn member_name(display_name: &str, email: &str) -> String {
    let name = display_name.trim();
    if !name.is_empty() {
        return name.to_string();
    }
    email.split('@').next().unwrap_or(email).to_string()
}

fn validate_name(name: &str) -> ApiResult<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
        return Err(ApiError::BadRequest(format!("family name must be 1 to {MAX_NAME_CHARS} characters")));
    }
    Ok(name.to_string())
}
