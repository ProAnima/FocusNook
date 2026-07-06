use crate::error::{AppError, AppResult};
use crate::state::AppState;
use axum::http::HeaderMap;
use base64::Engine;
use hmac::{Hmac, Mac};
use rand::RngCore;
use serde::Serialize;
use sha2::Sha256;
use subtle::ConstantTimeEq;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Debug)]
pub struct UserAuth {
    pub user_id: Uuid,
}

#[derive(Clone, Debug)]
pub struct DeviceAuth {
    pub user_id: Uuid,
    pub device_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssuedToken {
    pub token: String,
    pub token_hash: String,
}

pub fn issue_token(prefix: &str, pepper: &[u8]) -> AppResult<IssuedToken> {
    let mut bytes = [0_u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
    let token = format!("{prefix}_{encoded}");
    let token_hash = hash_token(&token, pepper)?;
    Ok(IssuedToken { token, token_hash })
}

pub fn hash_token(token: &str, pepper: &[u8]) -> AppResult<String> {
    let mut mac = HmacSha256::new_from_slice(pepper)
        .map_err(|_| AppError::Config("invalid token pepper".to_string()))?;
    mac.update(token.as_bytes());
    Ok(hex_encode(&mac.finalize().into_bytes()))
}

pub fn require_admin(headers: &HeaderMap, state: &AppState) -> AppResult<()> {
    let token = bearer_token(headers)?;
    let provided = hash_token(&token, &state.config.token_pepper)?;
    let expected = hash_token(&state.config.admin_token, &state.config.token_pepper)?;
    if provided.as_bytes().ct_eq(expected.as_bytes()).into() {
        Ok(())
    } else {
        Err(AppError::Unauthorized)
    }
}

pub async fn require_user(headers: &HeaderMap, state: &AppState) -> AppResult<UserAuth> {
    let token_hash = hash_token(&bearer_token(headers)?, &state.config.token_pepper)?;
    let user_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT u.id
         FROM user_tokens t
         JOIN users u ON u.id = t.user_id
         WHERE t.token_hash = $1
           AND t.revoked_at IS NULL
           AND u.disabled_at IS NULL",
    )
    .bind(token_hash)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::Unauthorized)?;
    Ok(UserAuth { user_id })
}

pub async fn require_device(headers: &HeaderMap, state: &AppState) -> AppResult<DeviceAuth> {
    let token_hash = hash_token(&bearer_token(headers)?, &state.config.token_pepper)?;
    let row = sqlx::query_as::<_, (Uuid, String)>(
        "UPDATE devices d
         SET last_seen_at = now()
         FROM users u
         WHERE d.user_id = u.id
           AND d.token_hash = $1
           AND d.revoked_at IS NULL
           AND u.disabled_at IS NULL
         RETURNING d.user_id, d.device_id",
    )
    .bind(token_hash)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::Unauthorized)?;
    Ok(DeviceAuth {
        user_id: row.0,
        device_id: row.1,
    })
}

fn bearer_token(headers: &HeaderMap) -> AppResult<String> {
    let raw = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or(AppError::Unauthorized)?;
    raw.strip_prefix("Bearer ")
        .map(str::to_string)
        .ok_or(AppError::Unauthorized)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_hash_uses_the_pepper() -> AppResult<()> {
        let one = hash_token("token", b"pepper-one")?;
        let two = hash_token("token", b"pepper-two")?;
        assert_ne!(one, two);
        Ok(())
    }
}
