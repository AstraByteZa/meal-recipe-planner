//! Accounts and bearer tokens. Passwords are hashed with argon2. Tokens are HS256 JWTs.

use rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::{FromRef, FromRequestParts, State};
use axum::http::header::AUTHORIZATION;
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::Json;
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult, internal};
use crate::state::AppState;

const TOKEN_DAYS: i64 = 30;
const MIN_PASSWORD_CHARS: usize = 8;
const MAX_PASSWORD_CHARS: usize = 1024;
const MAX_EMAIL_CHARS: usize = 254;
const MAX_DISPLAY_NAME_CHARS: usize = 80;

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: Uuid,
    exp: usize,
}

/// The signed-in user. Add it to a handler's arguments to require a valid bearer token.
pub struct AuthUser(pub Uuid);

impl<S> FromRequestParts<S> for AuthUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = AppState::from_ref(state);
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or(ApiError::Unauthorized)?;
        let claims = decode::<Claims>(token, &DecodingKey::from_secret(&state.jwt_secret), &Validation::default())
            .map_err(|_| ApiError::Unauthorized)?
            .claims;
        Ok(AuthUser(claims.sub))
    }
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub display_name: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct UpdateMeRequest {
    pub display_name: Option<String>,
}

#[derive(Clone, Serialize, FromRow)]
pub struct UserDto {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
}

#[derive(FromRow)]
struct UserRow {
    id: Uuid,
    email: String,
    password_hash: String,
    display_name: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserDto,
}

pub async fn register(
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> ApiResult<(StatusCode, Json<AuthResponse>)> {
    if !state.registration_open {
        return Err(ApiError::Forbidden("registration is closed on this server".to_string()));
    }
    let email = normalize_email(&request.email)?;
    check_password(&request.password)?;
    let display_name = request.display_name.trim().to_string();
    if display_name.chars().count() > MAX_DISPLAY_NAME_CHARS {
        return Err(ApiError::BadRequest(format!("display name is longer than {MAX_DISPLAY_NAME_CHARS} characters")));
    }

    let password_hash = hash_password(request.password).await?;
    let id = Uuid::new_v4();
    let inserted = sqlx::query("INSERT INTO users (id, email, password_hash, display_name) VALUES ($1, $2, $3, $4)")
        .bind(id)
        .bind(&email)
        .bind(&password_hash)
        .bind(&display_name)
        .execute(&state.pool)
        .await;
    match inserted {
        Ok(_) => {}
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => {
            return Err(ApiError::Conflict("that email is already registered".to_string()));
        }
        Err(error) => return Err(error.into()),
    }

    let user = UserDto { id, email, display_name };
    let token = issue_token(&state.jwt_secret, id)?;
    Ok((StatusCode::CREATED, Json(AuthResponse { token, user })))
}

pub async fn login(State(state): State<AppState>, Json(request): Json<LoginRequest>) -> ApiResult<Json<AuthResponse>> {
    let email = request.email.trim().to_lowercase();
    let row = sqlx::query_as::<_, UserRow>("SELECT id, email, password_hash, display_name FROM users WHERE email = $1")
        .bind(&email)
        .fetch_optional(&state.pool)
        .await?;
    let invalid = || ApiError::Unauthorized;
    let row = row.ok_or_else(invalid)?;
    if !verify_password(request.password, row.password_hash).await? {
        return Err(invalid());
    }

    let token = issue_token(&state.jwt_secret, row.id)?;
    let user = UserDto { id: row.id, email: row.email, display_name: row.display_name };
    Ok(Json(AuthResponse { token, user }))
}

pub async fn me(State(state): State<AppState>, AuthUser(me): AuthUser) -> ApiResult<Json<UserDto>> {
    Ok(Json(load_user(&state, me).await?))
}

pub async fn update_me(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Json(request): Json<UpdateMeRequest>,
) -> ApiResult<Json<UserDto>> {
    if let Some(name) = request.display_name {
        let name = name.trim();
        if name.chars().count() > MAX_DISPLAY_NAME_CHARS {
            return Err(ApiError::BadRequest(format!("display name is longer than {MAX_DISPLAY_NAME_CHARS} characters")));
        }
        sqlx::query("UPDATE users SET display_name = $2 WHERE id = $1")
            .bind(me)
            .bind(name)
            .execute(&state.pool)
            .await?;
    }
    Ok(Json(load_user(&state, me).await?))
}

async fn load_user(state: &AppState, id: Uuid) -> ApiResult<UserDto> {
    sqlx::query_as::<_, UserDto>("SELECT id, email, display_name FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(ApiError::Unauthorized)
}

fn normalize_email(email: &str) -> ApiResult<String> {
    let email = email.trim().to_lowercase();
    let valid = email.contains('@') && !email.starts_with('@') && !email.ends_with('@');
    if !valid || email.chars().count() > MAX_EMAIL_CHARS {
        return Err(ApiError::BadRequest("enter a valid email address".to_string()));
    }
    Ok(email)
}

fn check_password(password: &str) -> ApiResult<()> {
    let length = password.chars().count();
    if !(MIN_PASSWORD_CHARS..=MAX_PASSWORD_CHARS).contains(&length) {
        return Err(ApiError::BadRequest(format!(
            "password must be {MIN_PASSWORD_CHARS} to {MAX_PASSWORD_CHARS} characters"
        )));
    }
    Ok(())
}

pub fn issue_token(secret: &[u8], user: Uuid) -> ApiResult<String> {
    let exp = (Utc::now() + Duration::days(TOKEN_DAYS)).timestamp() as usize;
    encode(&Header::default(), &Claims { sub: user, exp }, &EncodingKey::from_secret(secret)).map_err(internal)
}

async fn hash_password(password: String) -> ApiResult<String> {
    // Argon2 is deliberately slow, so keep it off the async worker threads.
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default().hash_password(password.as_bytes(), &salt).map(|hash| hash.to_string())
    })
    .await
    .map_err(internal)?
    .map_err(internal)
}

async fn verify_password(password: String, hash: String) -> ApiResult<bool> {
    tokio::task::spawn_blocking(move || {
        let Ok(parsed) = PasswordHash::new(&hash) else {
            return false;
        };
        Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok()
    })
    .await
    .map_err(internal)
}
