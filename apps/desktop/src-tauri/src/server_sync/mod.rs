pub(crate) mod account;
pub(crate) mod account_auth;
mod apply;
mod apply_entities;
mod attachments;
mod credentials;
pub(crate) mod engine;
mod journal;
mod patch;
mod payload;
mod protocol;
#[cfg(test)]
mod test_support;
mod transport;

use std::sync::atomic::AtomicBool;

const MAX_OPS_PER_EXCHANGE: usize = 100;
const MAX_PENDING_OPERATION_SCAN: usize = MAX_OPS_PER_EXCHANGE * 10;
const MAX_EXCHANGE_ROUNDS: usize = 20;
const PERIODIC_SYNC_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);
const SERVER_EVENT_WAIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(25);
const SERVER_EVENT_ERROR_BACKOFF: std::time::Duration = std::time::Duration::from_secs(10);
const HTTP_CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
const HTTP_REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(45);
const BLOB_TRANSFER_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(180);
const FULL_RECONCILE_INTERVAL_SECONDS: i64 = 15 * 60;
const PRIVACY_POLICY_VERSION: &str = "2026-07-16";
static SYNC_IN_FLIGHT: AtomicBool = AtomicBool::new(false);
static SYNC_RERUN_REQUESTED: AtomicBool = AtomicBool::new(false);

#[cfg(feature = "cloud-providers")]
pub(crate) use account::ensure_local_device_id;
#[cfg(feature = "cloud-providers")]
pub(crate) use apply::{apply_remote_operation, reconcile_remote_reminder_alarm};
pub use attachments::ensure_audio_blob_downloaded;
pub use engine::{spawn_best_effort, spawn_periodic_best_effort, spawn_server_event_listener};
#[cfg(feature = "cloud-providers")]
pub(crate) use journal::{mark_synced, store_last_pulled_hlc, unsynced_operations};
#[cfg(feature = "cloud-providers")]
pub(crate) use patch::audio_blob_id_from_operation;
#[cfg(feature = "cloud-providers")]
pub(crate) use payload::remote_operation_from_local;
#[cfg(feature = "cloud-providers")]
pub(crate) use protocol::{LocalOperation, RemoteOperation};
