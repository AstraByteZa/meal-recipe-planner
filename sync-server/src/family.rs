//! Families: a group of users who share rows. Each family has an invite code that others use
//! to join.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

const MAX_NAME_CHARS: usize = 80;
/// Hex digits taken from a random UUID. 12 digits is 48 bits, which is too many to guess.
const INVITE_CODE_LENGTH: usize = 12;

#[derive(Serialize, FromRow)]
pub struct FamilyDto {
    pub id: Uuid,
    pub name: String,
    pub invite_code: String,
}

#[derive(Deserialize)]
pub struct CreateFamilyRequest {
    pub name: String,
}

#[derive(Deserialize)]
pub struct JoinFamilyRequest {
    pub invite_code: String,
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
    let families = sqlx::query_as::<_, FamilyDto>(
        "SELECT f.id, f.name, f.invite_code FROM families f \
         JOIN family_members m ON m.family_id = f.id WHERE m.user_id = $1 ORDER BY f.name",
    )
    .bind(me)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(families))
}

pub async fn create(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Json(request): Json<CreateFamilyRequest>,
) -> ApiResult<(StatusCode, Json<FamilyDto>)> {
    let name = request.name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
        return Err(ApiError::BadRequest(format!("family name must be 1 to {MAX_NAME_CHARS} characters")));
    }
    let family = FamilyDto {
        id: Uuid::new_v4(),
        name: name.to_string(),
        invite_code: invite_code(),
    };

    let mut tx = state.pool.begin().await?;
    sqlx::query("INSERT INTO families (id, name, invite_code) VALUES ($1, $2, $3)")
        .bind(family.id)
        .bind(&family.name)
        .bind(&family.invite_code)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO family_members (family_id, user_id) VALUES ($1, $2)")
        .bind(family.id)
        .bind(me)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(family)))
}

pub async fn join(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Json(request): Json<JoinFamilyRequest>,
) -> ApiResult<Json<FamilyDto>> {
    let code = request.invite_code.trim().to_uppercase();
    let family = sqlx::query_as::<_, FamilyDto>("SELECT id, name, invite_code FROM families WHERE invite_code = $1")
        .bind(&code)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| ApiError::NotFound("no family has that invite code".to_string()))?;

    sqlx::query("INSERT INTO family_members (family_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(family.id)
        .bind(me)
        .execute(&state.pool)
        .await?;
    Ok(Json(family))
}

fn invite_code() -> String {
    let hex = Uuid::new_v4().simple().to_string();
    hex[..INVITE_CODE_LENGTH].to_uppercase()
}
