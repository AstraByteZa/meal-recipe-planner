//! Membership changes after a join: an owner removes a member, anyone leaves, or the owner hands
//! the role to another member. The owner cannot leave until they hand the role over.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::Deserialize;
use shared_core::wire::FamilyRole;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::family::{member_ids, member_role, notify_membership, require_member, require_owner, role_to_db};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct TransferRequest {
    pub user_id: Uuid,
}

/// An owner removes another member. A member can remove only themself, which is leaving.
/// The removed person keeps their copy of the family's rows on their device as private data.
pub async fn remove(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Path((family, target)): Path<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    let mut tx = state.pool.begin().await?;
    let my_role = require_member(&mut tx, family, me).await?;
    if target != me && my_role != FamilyRole::Owner {
        return Err(ApiError::Forbidden("only the family owner can remove other members".to_string()));
    }
    let target_role = member_role(&mut tx, family, target)
        .await?
        .ok_or_else(|| ApiError::NotFound("that person is not in this family".to_string()))?;
    if target_role == FamilyRole::Owner {
        return Err(ApiError::Conflict(
            "the owner cannot leave or be removed. Make someone else the owner first, or delete the family".to_string(),
        ));
    }

    // Read the members before the delete, so the person being removed is still notified.
    let members = member_ids(&mut tx, family).await?;
    sqlx::query("DELETE FROM family_members WHERE family_id = $1 AND user_id = $2")
        .bind(family)
        .bind(target)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    notify_membership(&state, &members);
    Ok(StatusCode::NO_CONTENT)
}

/// The owner hands the role to another member. The old owner becomes a regular member.
pub async fn transfer(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Path(family): Path<Uuid>,
    Json(request): Json<TransferRequest>,
) -> ApiResult<StatusCode> {
    let mut tx = state.pool.begin().await?;
    require_owner(&mut tx, family, me).await?;
    if request.user_id == me {
        return Ok(StatusCode::NO_CONTENT);
    }
    member_role(&mut tx, family, request.user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("that person is not in this family".to_string()))?;

    // Demote first: the unique owner index allows only one owner row at a time.
    sqlx::query("UPDATE family_members SET role = $3 WHERE family_id = $1 AND user_id = $2")
        .bind(family)
        .bind(me)
        .bind(role_to_db(FamilyRole::Member))
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE family_members SET role = $3 WHERE family_id = $1 AND user_id = $2")
        .bind(family)
        .bind(request.user_id)
        .bind(role_to_db(FamilyRole::Owner))
        .execute(&mut *tx)
        .await?;
    let members = member_ids(&mut tx, family).await?;
    tx.commit().await?;
    notify_membership(&state, &members);
    Ok(StatusCode::NO_CONTENT)
}
