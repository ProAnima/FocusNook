use super::helpers::{hex_encode, validate_id};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Clone, Deserialize)]
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

pub(super) fn validate_operation(
    operation: &ClientOperation,
    expected_device_id: &str,
    state: &AppState,
) -> AppResult<()> {
    validate_id(&operation.operation_id, "operationId", 160)?;
    validate_id(&operation.device_id, "deviceId", 160)?;
    validate_id(&operation.entity_type, "entityType", 80)?;
    validate_id(&operation.entity_id, "entityId", 160)?;
    validate_id(&operation.op, "op", 80)?;
    validate_hlc(&operation.hlc)?;
    if operation.device_id != expected_device_id {
        return Err(AppError::Forbidden);
    }
    if operation.schema_version < 1 {
        return Err(AppError::BadRequest(
            "schemaVersion must be positive".to_string(),
        ));
    }
    if operation.payload_ciphertext.is_empty()
        || operation.payload_ciphertext.len() > state.config.max_operation_payload_bytes
    {
        return Err(AppError::BadRequest(
            "payloadCiphertext has invalid size".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn validate_hlc(value: &str) -> AppResult<()> {
    validate_id(value, "hlc", 220)?;
    let bytes = value.as_bytes();
    let has_min_shape = value.len() >= 31
        && bytes.get(4) == Some(&b'-')
        && bytes.get(7) == Some(&b'-')
        && bytes.get(10) == Some(&b'T')
        && bytes.get(13) == Some(&b':')
        && bytes.get(16) == Some(&b':')
        && bytes.get(19) == Some(&b'.')
        && bytes.get(23) == Some(&b'Z')
        && bytes.get(24) == Some(&b'-')
        && bytes.get(29) == Some(&b'-');
    if has_min_shape {
        Ok(())
    } else {
        Err(AppError::BadRequest("hlc is invalid".to_string()))
    }
}

pub(super) fn operation_digest(operation: &ClientOperation) -> String {
    let mut hasher = Sha256::new();
    hash_part(&mut hasher, &operation.operation_id);
    hash_part(&mut hasher, &operation.device_id);
    hash_part(&mut hasher, &operation.entity_type);
    hash_part(&mut hasher, &operation.entity_id);
    hash_part(&mut hasher, &operation.op);
    hash_part(&mut hasher, &operation.hlc);
    hasher.update(operation.schema_version.to_be_bytes());
    hash_part(&mut hasher, &operation.payload_ciphertext);
    hash_optional_part(&mut hasher, operation.payload_key_id.as_deref());
    hash_optional_part(&mut hasher, operation.payload_nonce.as_deref());
    hex_encode(&hasher.finalize())
}

fn hash_part(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value.as_bytes());
}

fn hash_optional_part(hasher: &mut Sha256, value: Option<&str>) {
    match value {
        Some(value) => {
            hasher.update([1]);
            hash_part(hasher, value);
        }
        None => hasher.update([0]),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn validates_expected_hlc_shape() {
        assert!(validate_hlc("2026-07-06T19:22:10.100Z-0001-device-a").is_ok());
        assert!(validate_hlc("2026-07-06 19:22:10").is_err());
    }

    #[test]
    fn operation_digest_changes_when_payload_changes() {
        let first = ClientOperation {
            device_id: "device".to_string(),
            entity_id: "note-1".to_string(),
            entity_type: "note".to_string(),
            hlc: "2026-07-06T19:22:10.100Z-0001-device".to_string(),
            op: "upsert".to_string(),
            operation_id: "op-1".to_string(),
            payload_ciphertext: "payload-a".to_string(),
            payload_key_id: Some("key-1".to_string()),
            payload_nonce: Some("nonce-1".to_string()),
            schema_version: 1,
        };
        let mut second = first.clone();
        second.payload_ciphertext = "payload-b".to_string();

        assert_ne!(operation_digest(&first), operation_digest(&second));
        assert_eq!(operation_digest(&first), operation_digest(&first));
    }
}
