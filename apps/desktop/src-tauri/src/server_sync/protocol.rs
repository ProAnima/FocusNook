use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RegisterDeviceRequest<'a> {
    pub(super) device_id: &'a str,
    pub(super) display_name: &'a str,
    pub(super) platform: &'a str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RegisterDeviceResponse {
    pub(super) device_token: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AccountAuthRequest<'a> {
    pub(super) email: &'a str,
    pub(super) password: &'a str,
    pub(super) display_name: Option<&'a str>,
    pub(super) device_id: &'a str,
    pub(super) device_name: &'a str,
    pub(super) platform: &'a str,
    pub(super) privacy_accepted: bool,
    pub(super) privacy_policy_version: Option<&'a str>,
}

#[derive(Serialize)]
pub(super) struct DeleteAccountRequest<'a> {
    pub(super) password: &'a str,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AccountAuthResponse {
    pub(super) device_token: String,
    pub(super) email: String,
    pub(super) display_name: String,
    pub(super) user_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerAccountRegistration {
    pub(super) display_name: String,
    pub(super) email: String,
    pub(super) password: String,
    pub(super) privacy_accepted: bool,
}

pub(super) struct ServerAccountSession {
    pub(super) email: String,
    pub(super) display_name: String,
    pub(super) user_id: String,
}

#[derive(Clone)]
pub(crate) struct LocalOperation {
    pub(crate) operation_id: String,
    pub(crate) device_id: String,
    pub(crate) entity_type: String,
    pub(crate) entity_id: String,
    pub(crate) op: String,
    pub(crate) patch: String,
    pub(crate) hlc: String,
    pub(crate) schema_version: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SyncExchangeRequest {
    pub(super) device_id: String,
    pub(super) last_pulled_hlc: Option<String>,
    pub(super) operations: Vec<ClientOperation>,
    pub(super) profile_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ClientOperation {
    pub(super) device_id: String,
    pub(super) entity_id: String,
    pub(super) entity_type: String,
    pub(super) hlc: String,
    pub(super) op: String,
    pub(super) operation_id: String,
    pub(super) payload_ciphertext: String,
    pub(super) payload_key_id: Option<String>,
    pub(super) payload_nonce: Option<String>,
    pub(super) schema_version: i32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SyncExchangeResponse {
    pub(super) accepted_count: i64,
    pub(super) duplicate_count: i64,
    pub(super) next_pull_hlc: Option<String>,
    pub(super) operations: Vec<RemoteOperation>,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemoteOperation {
    pub(crate) device_id: String,
    pub(crate) entity_id: String,
    pub(crate) entity_type: String,
    pub(crate) hlc: String,
    pub(crate) op: String,
    pub(crate) operation_id: String,
    pub(crate) payload_ciphertext: String,
    pub(crate) schema_version: i32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct UploadBlobResponse {
    pub(super) blob_id: String,
    pub(super) size_bytes: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SyncEventResponse {
    pub(super) changed: bool,
    #[serde(default)]
    pub(super) sequence: u64,
}
