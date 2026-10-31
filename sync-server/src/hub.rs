//! In-process notifications for `/sync/stream`. Each channel is a user (private ops) or a family
//! (shared ops). This is single-instance: a second server replica would not see these sends.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use tokio::sync::broadcast;
use uuid::Uuid;

/// How many notifications a slow stream can fall behind before it gets `Lagged`. A lagged
/// stream still gets an event, because the client pulls anyway.
const CHANNEL_CAPACITY: usize = 64;

#[derive(Clone, Default)]
pub struct Hub {
    channels: Arc<Mutex<HashMap<Uuid, broadcast::Sender<i64>>>>,
}

impl Hub {
    pub fn subscribe(&self, channel: Uuid) -> broadcast::Receiver<i64> {
        self.sender(channel).subscribe()
    }

    /// Tells subscribers of `channel` that the log has reached `seq`. Nobody listening is fine.
    pub fn notify(&self, channel: Uuid, seq: i64) {
        let channels = self.channels.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(sender) = channels.get(&channel) {
            let _ = sender.send(seq);
        }
    }

    fn sender(&self, channel: Uuid) -> broadcast::Sender<i64> {
        let mut channels = self.channels.lock().unwrap_or_else(PoisonError::into_inner);
        channels.entry(channel).or_insert_with(|| broadcast::channel(CHANNEL_CAPACITY).0).clone()
    }
}
