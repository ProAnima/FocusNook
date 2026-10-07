#[cfg(feature = "cloud-providers")]
use super::protocol::RemoteOperation;
use super::protocol::{ClientOperation, LocalOperation};
use crate::blob_crypto;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::Value;

// P0 аудит: раньше payload_ciphertext буквально был operation.patch — открытый
// JSON задачи/заметки/напоминания, никакого шифрования, несмотря на название
// поля. Теперь шифруем тем же AES-256-GCM/media_key, что уже используется для
// вложений (blob_crypto.rs) — не новая криптосхема, тот же примитив на новый
// тип данных. media_key отсутствует только у профилей, подключённых через
// старый admin-issued userToken без email/password логина (см.
// account_auth.rs) — для них операции остаются как раньше, без шифрования,
// а не ломаются; это тот же компромисс, что уже принят для blob-вложений
// (сравни credentials.media_key.as_deref() в attachments::upload_pending_blobs).
fn encode_operation_payload(media_key: Option<&str>, patch: &str) -> Result<String, String> {
    match media_key {
        Some(key) => {
            blob_crypto::encrypt(key, patch.as_bytes()).map(|bytes| STANDARD.encode(bytes))
        }
        None => Ok(patch.to_string()),
    }
}

// Порядок проверки: сначала пробуем расшифровать (валидный base64 + верный
// magic + успешный AEAD-тег) — если что-то из этого не сошлось (нет
// media_key, старые незашифрованные данные, чужой ключ), падаем обратно на
// "payload_ciphertext это и есть открытый JSON". Совпадение случайно
// выглядящих plaintext-байт с валидным шифротекстом того же ключа
// статистически исключено GCM-тегом, так что порядок проверки безопасен для
// обратной совместимости со всем, что уже успело уйти на VDS/Google Drive
// без шифрования до этого фикса.
pub(super) fn decode_operation_payload(
    media_key: Option<&str>,
    payload_ciphertext: &str,
) -> Result<Value, String> {
    let decrypted = media_key.and_then(|key| {
        STANDARD
            .decode(payload_ciphertext.trim())
            .ok()
            .and_then(|bytes| blob_crypto::decrypt(key, &bytes).ok())
    });
    let json_bytes = decrypted.unwrap_or_else(|| payload_ciphertext.as_bytes().to_vec());
    serde_json::from_slice::<Value>(&json_bytes)
        .map_err(|e| format!("remote operation payload is invalid json: {e}"))
}

#[cfg(feature = "cloud-providers")]
pub(crate) fn remote_operation_from_local(
    operation: &LocalOperation,
    media_key: Option<&str>,
) -> Result<RemoteOperation, String> {
    Ok(RemoteOperation {
        device_id: operation.device_id.clone(),
        entity_id: operation.entity_id.clone(),
        entity_type: operation.entity_type.clone(),
        hlc: operation.hlc.clone(),
        op: operation.op.clone(),
        operation_id: operation.operation_id.clone(),
        payload_ciphertext: encode_operation_payload(media_key, &operation.patch)?,
        schema_version: operation.schema_version,
    })
}

pub(super) fn client_operation(
    operation: &LocalOperation,
    media_key: Option<&str>,
) -> Result<ClientOperation, String> {
    Ok(ClientOperation {
        device_id: operation.device_id.clone(),
        entity_id: operation.entity_id.clone(),
        entity_type: operation.entity_type.clone(),
        hlc: operation.hlc.clone(),
        op: operation.op.clone(),
        operation_id: operation.operation_id.clone(),
        payload_ciphertext: encode_operation_payload(media_key, &operation.patch)?,
        payload_key_id: None,
        payload_nonce: None,
        schema_version: operation.schema_version,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    // P0 аудит: раньше payload_ciphertext был буквально operation.patch,
    // никакого шифрования не было несмотря на название поля. Эти тесты
    // проверяют, что сейчас это не так, и что старые незашифрованные данные
    // (уже ушедшие на VDS/Google Drive до этого фикса) продолжают читаться.
    #[test]
    fn encode_operation_payload_encrypts_when_media_key_present() {
        let patch = r#"{"title":"Buy groceries","status":"open"}"#;
        let encoded = encode_operation_payload(Some("test-media-key"), patch).unwrap();
        assert_ne!(encoded, patch);
        assert!(!encoded.contains("Buy groceries"));
    }

    #[test]
    fn encode_operation_payload_is_plaintext_passthrough_without_media_key() {
        let patch = r#"{"title":"Buy groceries"}"#;
        assert_eq!(encode_operation_payload(None, patch).unwrap(), patch);
    }

    #[test]
    fn decode_operation_payload_round_trips_through_encode() -> Result<(), String> {
        let patch = serde_json::json!({"title": "Buy groceries", "status": "open"});
        let encoded = encode_operation_payload(Some("test-media-key"), &patch.to_string()).unwrap();

        let decoded = decode_operation_payload(Some("test-media-key"), &encoded)?;

        assert_eq!(decoded, patch);
        Ok(())
    }

    #[test]
    fn decode_operation_payload_falls_back_to_plaintext_for_legacy_data() -> Result<(), String> {
        let legacy_patch = serde_json::json!({"status": "done"});

        let decoded = decode_operation_payload(Some("test-media-key"), &legacy_patch.to_string())?;

        assert_eq!(decoded, legacy_patch);
        Ok(())
    }

    #[test]
    fn decode_operation_payload_with_wrong_media_key_does_not_leak_plaintext() {
        let patch = serde_json::json!({"title": "Buy groceries"});
        let encoded = encode_operation_payload(Some("correct-key"), &patch.to_string()).unwrap();

        // С неправильным ключом это не откатится на "как есть = plaintext",
        // потому что валидный base64/magic-конверт с чужим ключом это не тот
        // же случай, что и настоящий legacy plaintext — GCM-тег не совпадёт,
        // decrypt провалится, а сам base64-текст не распарсится как JSON.
        let result = decode_operation_payload(Some("wrong-key"), &encoded);

        assert!(result.is_err());
    }
}
