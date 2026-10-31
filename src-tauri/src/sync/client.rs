//! HTTP calls to the sync server. Each method is one endpoint. Failures come back as `SyncError`,
//! which says whether retrying can help.

use std::fmt;
use std::time::Duration;

use reqwest::{Client, Response, StatusCode};
use serde::Deserialize;
use serde_json::{Value, json};
use shared_core::wire::{PullResponse, PushRequest, PushResponse, SseEvent};

use crate::error::AppError;

/// The server sends a keep-alive every 15 seconds, so silence for this long means the stream is dead.
const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(45);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub enum SyncError {
    /// The token is missing or no longer valid. Retrying will not help until the user signs in.
    Unauthorized(String),
    /// The server refused the request itself. Retrying the same request will not help.
    Rejected(String),
    /// The server could not be reached, or it failed. Retrying later may help.
    Unreachable(String),
    /// A problem on this device, such as a database error.
    Local(AppError),
}

impl fmt::Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncError::Unauthorized(message) | SyncError::Rejected(message) | SyncError::Unreachable(message) => {
                f.write_str(message)
            }
            SyncError::Local(error) => write!(f, "{error}"),
        }
    }
}

impl From<AppError> for SyncError {
    fn from(error: AppError) -> Self {
        SyncError::Local(error)
    }
}

impl From<sqlx::Error> for SyncError {
    fn from(error: sqlx::Error) -> Self {
        SyncError::Local(error.into())
    }
}

impl From<SyncError> for AppError {
    fn from(error: SyncError) -> Self {
        match error {
            SyncError::Local(error) => error,
            other => AppError::Platform(other.to_string()),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserDto {
    pub id: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserDto,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FamilyDto {
    pub id: String,
    pub name: String,
    pub invite_code: String,
}

/// A client for one server. `token` is needed for everything except register and login.
#[derive(Clone)]
pub struct Api {
    http: Client,
    base: String,
    token: Option<String>,
}

impl Api {
    pub fn new(base: &str, token: Option<&str>) -> Result<Self, SyncError> {
        // No overall timeout here: the stream request stays open. Other requests set their own.
        let http = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|error| SyncError::Unreachable(error.to_string()))?;
        Ok(Self {
            http,
            base: base.trim_end_matches('/').to_string(),
            token: token.map(str::to_string),
        })
    }

    pub async fn register(&self, email: &str, password: &str) -> Result<AuthResponse, SyncError> {
        self.post("/auth/register", &json!({ "email": email, "password": password })).await
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<AuthResponse, SyncError> {
        self.post("/auth/login", &json!({ "email": email, "password": password })).await
    }

    pub async fn create_family(&self, name: &str) -> Result<FamilyDto, SyncError> {
        self.post("/family/create", &json!({ "name": name })).await
    }

    pub async fn join_family(&self, invite_code: &str) -> Result<FamilyDto, SyncError> {
        self.post("/family/join", &json!({ "invite_code": invite_code })).await
    }

    pub async fn push(&self, request: &PushRequest) -> Result<PushResponse, SyncError> {
        self.post("/sync/push", request).await
    }

    pub async fn pull(&self, since: i64, limit: i64) -> Result<PullResponse, SyncError> {
        let response = self
            .authorized(self.http.get(self.url(&format!("/sync/pull?since={since}&limit={limit}"))))
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(transport)?;
        read_json(check(response).await?).await
    }

    /// Opens `/sync/stream`. Each event only says that something changed, so the caller pulls.
    pub async fn stream(&self) -> Result<EventStream, SyncError> {
        let response = self
            .authorized(self.http.get(self.url("/sync/stream")))
            .send()
            .await
            .map_err(transport)?;
        Ok(EventStream { response: check(response).await?, buffer: Vec::new() })
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    fn authorized(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.token {
            Some(token) => request.bearer_auth(token),
            None => request,
        }
    }

    async fn post<B, T>(&self, path: &str, body: &B) -> Result<T, SyncError>
    where
        B: serde::Serialize + ?Sized,
        T: for<'de> Deserialize<'de>,
    {
        let response = self
            .authorized(self.http.post(self.url(path)))
            .json(body)
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(transport)?;
        read_json(check(response).await?).await
    }
}

/// The open `/sync/stream` response, read one event at a time.
pub struct EventStream {
    response: Response,
    buffer: Vec<u8>,
}

impl EventStream {
    /// Waits for the next `op_ready` event. Errors when the stream ends or goes silent.
    pub async fn next_event(&mut self) -> Result<SseEvent, SyncError> {
        loop {
            if let Some(event) = take_event(&mut self.buffer) {
                return Ok(event);
            }
            let chunk = tokio::time::timeout(STREAM_IDLE_TIMEOUT, self.response.chunk())
                .await
                .map_err(|_| SyncError::Unreachable("sync stream went silent".to_string()))?
                .map_err(transport)?;
            match chunk {
                Some(bytes) => self.buffer.extend_from_slice(&bytes),
                None => return Err(SyncError::Unreachable("sync stream closed".to_string())),
            }
        }
    }
}

/// Removes the next complete `data:` event from the buffer. Comments (keep-alives) and blocks
/// without a valid event are dropped along the way. Returns `None` when no complete block is left.
fn take_event(buffer: &mut Vec<u8>) -> Option<SseEvent> {
    while let Some(end) = buffer.windows(2).position(|pair| pair == b"\n\n") {
        let block: Vec<u8> = buffer.drain(..end + 2).collect();
        let text = String::from_utf8_lossy(&block);
        for line in text.lines() {
            if let Some(data) = line.strip_prefix("data:") {
                if let Ok(event) = serde_json::from_str::<SseEvent>(data.trim()) {
                    return Some(event);
                }
            }
        }
    }
    None
}

fn transport(error: reqwest::Error) -> SyncError {
    SyncError::Unreachable(error.to_string())
}

/// Turns a non-success response into a `SyncError`, using the server's `{"error": ...}` message.
async fn check(response: Response) -> Result<Response, SyncError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let body: Value = response.json().await.unwrap_or(Value::Null);
    let message = body["error"]
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| format!("sync server returned {status}"));
    Err(match status {
        StatusCode::UNAUTHORIZED => SyncError::Unauthorized(message),
        status if status.is_server_error() => SyncError::Unreachable(message),
        _ => SyncError::Rejected(message),
    })
}

async fn read_json<T: for<'de> Deserialize<'de>>(response: Response) -> Result<T, SyncError> {
    response
        .json()
        .await
        .map_err(|error| SyncError::Unreachable(format!("unexpected response from sync server: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EVENT: &str = r#"data: {"type":"op_ready","seq":7}"#;

    #[test]
    fn parses_events_and_skips_keep_alives() {
        let mut buffer = format!(": keep-alive\n\n{EVENT}\n\n").into_bytes();
        let event = take_event(&mut buffer).unwrap();
        assert_eq!(event, SseEvent::op_ready(7));
        assert!(take_event(&mut buffer).is_none());
    }

    #[test]
    fn waits_for_a_complete_block() {
        let mut buffer = format!("{EVENT}\n").into_bytes();
        assert!(take_event(&mut buffer).is_none());
        buffer.extend_from_slice(b"\n");
        assert_eq!(take_event(&mut buffer), Some(SseEvent::op_ready(7)));
    }

    #[test]
    fn splits_a_multibyte_character_across_chunks() {
        // Byte-level buffering means a chunk may end inside a character. The block is only
        // decoded once it is complete.
        let mut buffer = b"data: {\"type\":\"op_ready\",\"seq\":3,\"note\":\"\xc3".to_vec();
        assert!(take_event(&mut buffer).is_none());
        buffer.extend_from_slice(b"\xa9\"}\n\n");
        assert_eq!(take_event(&mut buffer), Some(SseEvent::op_ready(3)));
    }
}
