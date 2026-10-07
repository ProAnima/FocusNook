use super::payload::decode_operation_payload;
use super::protocol::RemoteOperation;
use serde_json::Value;

pub(super) fn string_field<'a>(patch: &'a Value, key: &str) -> Option<&'a str> {
    patch.get(key).and_then(Value::as_str)
}

pub(super) fn nullable_string_field(patch: &Value, key: &str) -> Option<Option<String>> {
    patch.get(key).map(|value| {
        if value.is_null() {
            None
        } else {
            value.as_str().map(str::to_string)
        }
    })
}

pub(super) fn bool_field(patch: &Value, key: &str) -> Option<bool> {
    patch.get(key).and_then(Value::as_bool)
}

pub(super) fn attachment_refs_from_patch(patch: &Value) -> Vec<(String, String)> {
    let mut refs = string_field(patch, "audioPath")
        .map(|id| (id.to_string(), "audio/webm".to_string()))
        .into_iter()
        .collect::<Vec<_>>();
    if let Some(items) = patch.get("attachmentIds").and_then(Value::as_array) {
        refs.extend(
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|id| (id.to_string(), "application/octet-stream".to_string())),
        );
    }
    if let Some(items) = patch.get("attachments").and_then(Value::as_array) {
        refs.extend(items.iter().filter_map(|item| {
            let id = item
                .as_str()
                .or_else(|| item.get("id").and_then(Value::as_str))?;
            let content_type = item
                .get("contentType")
                .and_then(Value::as_str)
                .unwrap_or("application/octet-stream");
            Some((id.to_string(), content_type.to_string()))
        }));
    }
    refs.sort_by(|left, right| left.0.cmp(&right.0));
    refs.dedup_by(|left, right| left.0 == right.0);
    refs
}

#[cfg(feature = "cloud-providers")]
pub(crate) fn audio_blob_id_from_operation(
    operation: &RemoteOperation,
    media_key: Option<&str>,
) -> Option<String> {
    if !matches!(operation.entity_type.as_str(), "note" | "reminder") || operation.op == "delete" {
        return None;
    }
    let patch = decode_operation_payload(media_key, &operation.payload_ciphertext).ok()?;
    string_field(&patch, "audioPath").map(str::to_string)
}

pub(super) fn attachment_refs_from_operation(
    operation: &RemoteOperation,
    media_key: Option<&str>,
) -> Vec<(String, String)> {
    if operation.op == "delete" {
        return Vec::new();
    }
    decode_operation_payload(media_key, &operation.payload_ciphertext)
        .map(|patch| attachment_refs_from_patch(&patch))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn attachment_contract_supports_audio_and_future_images() {
        let refs = attachment_refs_from_patch(&serde_json::json!({
            "audioPath": "voice.webm",
            "attachments": [
                {"id": "picture.png", "contentType": "image/png"},
                {"id": "picture.png", "contentType": "image/png"}
            ]
        }));

        assert_eq!(
            refs,
            vec![
                ("picture.png".to_string(), "image/png".to_string()),
                ("voice.webm".to_string(), "audio/webm".to_string())
            ]
        );
    }
}
