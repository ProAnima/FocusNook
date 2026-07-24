use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::broadcast;
use tokio::time::{timeout, Duration};
use uuid::Uuid;

const CHANNEL_CAPACITY: usize = 64;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncEvent {
    pub sequence: u64,
    pub reason: String,
}

#[derive(Default)]
pub struct SyncEventHub {
    inner: Mutex<SyncEventHubInner>,
}

#[derive(Default)]
struct SyncEventHubInner {
    channels: HashMap<Uuid, broadcast::Sender<SyncEvent>>,
    sequences: HashMap<Uuid, u64>,
}

impl SyncEventHub {
    pub fn notify(&self, user_id: Uuid, reason: &'static str) {
        let (sender, event) = {
            let mut inner = match self.inner.lock() {
                Ok(inner) => inner,
                Err(_) => return,
            };
            let next = inner
                .sequences
                .entry(user_id)
                .and_modify(|value| *value = value.saturating_add(1))
                .or_insert(1);
            let event = SyncEvent {
                sequence: *next,
                reason: reason.to_string(),
            };
            let sender = inner
                .channels
                .entry(user_id)
                .or_insert_with(|| broadcast::channel(CHANNEL_CAPACITY).0)
                .clone();
            (sender, event)
        };
        let _ = sender.send(event);
    }

    pub async fn wait_after(
        &self,
        user_id: Uuid,
        after_sequence: u64,
        wait_for: Duration,
    ) -> Option<SyncEvent> {
        let mut receiver = {
            let mut inner = self.inner.lock().ok()?;
            let current_sequence = inner.sequences.get(&user_id).copied().unwrap_or(0);
            let receiver = inner
                .channels
                .entry(user_id)
                .or_insert_with(|| broadcast::channel(CHANNEL_CAPACITY).0)
                .subscribe();
            if current_sequence > after_sequence {
                return Some(SyncEvent {
                    sequence: current_sequence,
                    reason: "catch-up".to_string(),
                });
            }
            receiver
        };

        match timeout(wait_for, receiver.recv()).await {
            Ok(Ok(event)) => Some(event),
            Ok(Err(broadcast::error::RecvError::Lagged(_))) => {
                let sequence = self.current_sequence(user_id);
                Some(SyncEvent {
                    sequence,
                    reason: "lagged".to_string(),
                })
            }
            _ => None,
        }
    }

    fn current_sequence(&self, user_id: Uuid) -> u64 {
        self.inner
            .lock()
            .ok()
            .and_then(|inner| inner.sequences.get(&user_id).copied())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn wait_returns_notified_event() -> Result<(), Box<dyn std::error::Error>> {
        let hub = std::sync::Arc::new(SyncEventHub::default());
        let user_id = Uuid::now_v7();
        let waiter = {
            let hub = hub.clone();
            tokio::spawn(async move { hub.wait_after(user_id, 0, Duration::from_secs(1)).await })
        };
        tokio::task::yield_now().await;
        hub.notify(user_id, "operation");

        let event = waiter.await?.ok_or("event timed out")?;

        assert_eq!(event.sequence, 1);
        assert_eq!(event.reason, "operation");
        Ok(())
    }

    #[tokio::test]
    async fn wait_times_out_without_event() {
        let hub = SyncEventHub::default();

        let event = hub
            .wait_after(Uuid::now_v7(), 0, Duration::from_millis(1))
            .await;

        assert!(event.is_none());
    }

    #[tokio::test]
    async fn wait_after_catches_an_event_published_before_subscription() {
        let hub = SyncEventHub::default();
        let user_id = Uuid::now_v7();
        hub.notify(user_id, "operation");

        let event = hub.wait_after(user_id, 0, Duration::from_millis(1)).await;

        assert_eq!(event.map(|value| value.sequence), Some(1));
    }
}
