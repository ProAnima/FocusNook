use crate::error::{AppError, AppResult};
use crate::state::AppState;
use axum::http::HeaderMap;
use uuid::Uuid;

pub(super) async fn record_traffic(
    state: &AppState,
    user_id: Uuid,
    inbound_bytes: i64,
    outbound_bytes: i64,
) -> AppResult<()> {
    if inbound_bytes == 0 && outbound_bytes == 0 {
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO sync_traffic_counters (user_id, inbound_bytes, outbound_bytes, updated_at)
         VALUES ($1, $2, $3, now())
         ON CONFLICT (user_id)
         DO UPDATE SET
           inbound_bytes = sync_traffic_counters.inbound_bytes + excluded.inbound_bytes,
           outbound_bytes = sync_traffic_counters.outbound_bytes + excluded.outbound_bytes,
           updated_at = now()",
    )
    .bind(user_id)
    .bind(inbound_bytes)
    .bind(outbound_bytes)
    .execute(&state.pool)
    .await?;
    Ok(())
}

pub(super) fn canonical_profile_id(user_id: Uuid) -> String {
    user_id.to_string()
}

pub(super) fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(super) fn validate_id(value: &str, field: &str, max_len: usize) -> AppResult<()> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > max_len || trimmed.chars().any(char::is_whitespace) {
        return Err(AppError::BadRequest(format!("{field} is invalid")));
    }
    Ok(())
}

pub(super) fn validate_label(value: &str, field: &str, max_len: usize) -> AppResult<()> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > max_len {
        return Err(AppError::BadRequest(format!("{field} is invalid")));
    }
    Ok(())
}

// P0 аудит: раньше X-Forwarded-For проверялся первым и брался первым
// значением из списка — а nginx (см. focusnook.conf, $proxy_add_x_forwarded_for)
// дописывает настоящий $remote_addr В КОНЕЦ существующего заголовка, а не
// заменяет его, так что первое значение остаётся полностью под контролем
// клиента. Любой желающий обойти rate-limit на логин/регистрацию просто
// присылал свой X-Forwarded-For на каждый запрос.
//
// X-Real-IP — не то же самое: nginx выставляет его через `proxy_set_header
// X-Real-IP $remote_addr`, что заменяет любое клиентское значение целиком, а
// не дописывает к нему. Он всегда достоверен для этого конкретного nginx-фронта,
// поэтому теперь основной источник — он; X-Forwarded-For остаётся только
// запасным вариантом для деплоя без X-Real-IP (например, прямого доступа в
// дев-окружении без nginx), где он настолько же спуфится, что и раньше.
pub(super) fn client_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-real-ip")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            headers
                .get("x-forwarded-for")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.split(',').next())
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn rejects_whitespace_in_identifiers() {
        assert!(validate_id("abc def", "field", 20).is_err());
        assert!(validate_id("abcdef", "field", 20).is_ok());
    }

    fn headers_with(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.insert(
                axum::http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                value.parse().unwrap(),
            );
        }
        headers
    }

    // P0 аудит: nginx дописывает $remote_addr в конец X-Forwarded-For, а не
    // заменяет его — так что клиент, приславший свой X-Forwarded-For, не
    // может подменить X-Real-IP тем же трюком (nginx выставляет его через
    // proxy_set_header, который заменяет значение целиком).
    #[test]
    fn client_ip_prefers_x_real_ip_over_a_spoofed_x_forwarded_for() {
        let headers = headers_with(&[("x-forwarded-for", "1.2.3.4"), ("x-real-ip", "203.0.113.9")]);
        assert_eq!(client_ip(&headers), "203.0.113.9");
    }

    #[test]
    fn client_ip_falls_back_to_x_forwarded_for_without_x_real_ip() {
        let headers = headers_with(&[("x-forwarded-for", "203.0.113.9, 10.0.0.1")]);
        assert_eq!(client_ip(&headers), "203.0.113.9");
    }

    #[test]
    fn client_ip_is_unknown_without_either_header() {
        assert_eq!(client_ip(&HeaderMap::new()), "unknown");
    }
}
