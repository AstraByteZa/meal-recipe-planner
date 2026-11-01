//! Limits failed invite attempts per user. Counted in memory, like the hub, so a restart clears it.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

const WINDOW: Duration = Duration::from_secs(15 * 60);
/// Failed attempts allowed in one window. The next attempt after this many gets a 429.
const MAX_FAILURES: usize = 10;

#[derive(Clone, Default)]
pub struct JoinThrottle {
    failures: Arc<Mutex<HashMap<Uuid, VecDeque<Instant>>>>,
}

impl JoinThrottle {
    /// Fails with 429 if `user` has used up their failed attempts in the current window.
    pub fn check(&self, user: Uuid) -> ApiResult<()> {
        let mut failures = self.failures.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(recent) = failures.get_mut(&user) else {
            return Ok(());
        };
        prune(recent);
        if recent.len() >= MAX_FAILURES {
            return Err(ApiError::TooManyRequests(
                "too many invite attempts. Wait a few minutes and try again".to_string(),
            ));
        }
        Ok(())
    }

    /// Counts one failed attempt for `user`.
    pub fn record_failure(&self, user: Uuid) {
        let mut failures = self.failures.lock().unwrap_or_else(PoisonError::into_inner);
        let recent = failures.entry(user).or_default();
        prune(recent);
        recent.push_back(Instant::now());
    }
}

fn prune(recent: &mut VecDeque<Instant>) {
    while recent.front().is_some_and(|at| at.elapsed() > WINDOW) {
        recent.pop_front();
    }
}
