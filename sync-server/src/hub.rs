//! In-process notifications for `/sync/stream`. Each channel is a user (private ops and their own
//! membership) or a family (shared ops). This is single-instance: a second server replica would
//! not see these sends.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use tokio::sync::broadcast;
use uuid::Uuid;

/// How many notifications a slow stream can fall behind before it gets `Lagged`. A lagged
/// stream still gets an event, because the client pulls anyway.
const CHANNEL_CAPACITY: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Notice {
    /// The op log has reached this seq on the channel.
    Ops(i64),
    /// The listener's family membership changed, so it should re-read its families.
    Membership,
}

#[derive(Clone, Default)]
pub struct Hub {
    channels: Arc<Mutex<HashMap<Uuid, broadcast::Sender<Notice>>>>,
}

impl Hub {
    pub fn subscribe(&self, channel: Uuid) -> broadcast::Receiver<Notice> {
        self.sender(channel).subscribe()
    }

    /// Tells subscribers of `channel` about `notice`. Nobody listening is fine.
    pub fn notify(&self, channel: Uuid, notice: Notice) {
        let channels = self.channels.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(sender) = channels.get(&channel) {
            let _ = sender.send(notice);
        }
    }

    fn sender(&self, channel: Uuid) -> broadcast::Sender<Notice> {
        let mut channels = self.channels.lock().unwrap_or_else(PoisonError::into_inner);
        channels.entry(channel).or_insert_with(|| broadcast::channel(CHANNEL_CAPACITY).0).clone()
    }
}
