use crate::error::{AppError, AppResult};
use crate::legal::LegalIdentity;
use base64::Engine;
use std::net::SocketAddr;

const MAX_CONFIGURED_BLOB_BYTES: usize = 64 * 1024 * 1024;
const MAX_CONFIGURED_OPERATION_BYTES: usize = 1024 * 1024;
const MAX_CONFIGURED_REQUEST_BYTES: usize = 128 * 1024 * 1024;
const MAX_CONFIGURED_OPS: i64 = 1000;
const REQUEST_JSON_OVERHEAD_BYTES: usize = 1024 * 1024;

#[derive(Clone)]
pub struct Config {
    pub admin_token: String,
    pub admin_web_password: String,
    pub bind_addr: SocketAddr,
    pub database_url: String,
    pub encryption_key: Vec<u8>,
    pub max_blob_bytes: usize,
    pub max_operation_payload_bytes: usize,
    pub max_ops_per_exchange: i64,
    pub max_request_body_bytes: usize,
    pub legal_identity: Option<LegalIdentity>,
    pub public_base_url: String,
    pub token_pepper: Vec<u8>,
}

impl Config {
    pub fn from_env() -> AppResult<Self> {
        let max_blob_bytes = bounded_usize(
            "FOCUSNOOK_MAX_BLOB_BYTES",
            "15728640",
            MAX_CONFIGURED_BLOB_BYTES,
        )?;
        let max_request_body_bytes = bounded_usize(
            "FOCUSNOOK_MAX_REQUEST_BODY_BYTES",
            "33554432",
            MAX_CONFIGURED_REQUEST_BYTES,
        )?;
        validate_blob_request_limit(max_blob_bytes, max_request_body_bytes)?;
        Ok(Self {
            admin_token: required("FOCUSNOOK_ADMIN_TOKEN")?,
            admin_web_password: required("FOCUSNOOK_WEB_SECONDARY_PASSWORD")?,
            bind_addr: optional("FOCUSNOOK_BIND_ADDR", "0.0.0.0:8080").parse()?,
            database_url: required("FOCUSNOOK_DATABASE_URL")?,
            encryption_key: decode_key("FOCUSNOOK_ENCRYPTION_KEY_B64")?,
            max_blob_bytes,
            max_operation_payload_bytes: bounded_usize(
                "FOCUSNOOK_MAX_OPERATION_PAYLOAD_BYTES",
                "262144",
                MAX_CONFIGURED_OPERATION_BYTES,
            )?,
            max_ops_per_exchange: bounded_i64(
                "FOCUSNOOK_MAX_OPS_PER_EXCHANGE",
                "500",
                MAX_CONFIGURED_OPS,
            )?,
            max_request_body_bytes,
            legal_identity: optional_legal_identity()?,
            public_base_url: optional("FOCUSNOOK_PUBLIC_BASE_URL", "http://localhost:8080"),
            token_pepper: decode_key("FOCUSNOOK_TOKEN_PEPPER_B64")?,
        })
    }
}

fn bounded_usize(name: &str, default: &str, maximum: usize) -> AppResult<usize> {
    let value = optional(name, default).parse::<usize>()?;
    if value == 0 || value > maximum {
        return Err(AppError::Config(format!(
            "{name} must be between 1 and {maximum}"
        )));
    }
    Ok(value)
}

fn bounded_i64(name: &str, default: &str, maximum: i64) -> AppResult<i64> {
    let value = optional(name, default).parse::<i64>()?;
    if value <= 0 || value > maximum {
        return Err(AppError::Config(format!(
            "{name} must be between 1 and {maximum}"
        )));
    }
    Ok(value)
}

fn minimum_blob_request_bytes(max_blob_bytes: usize) -> AppResult<usize> {
    max_blob_bytes
        .checked_add(2)
        .and_then(|value| value.checked_div(3))
        .and_then(|value| value.checked_mul(4))
        .and_then(|value| value.checked_add(REQUEST_JSON_OVERHEAD_BYTES))
        .ok_or_else(|| AppError::Config("blob request limit overflow".to_string()))
}

fn validate_blob_request_limit(max_blob_bytes: usize, max_request_bytes: usize) -> AppResult<()> {
    let minimum = minimum_blob_request_bytes(max_blob_bytes)?;
    if max_request_bytes < minimum {
        return Err(AppError::Config(format!(
            "FOCUSNOOK_MAX_REQUEST_BODY_BYTES must be at least {minimum} for the configured blob limit"
        )));
    }
    Ok(())
}

fn optional_legal_identity() -> AppResult<Option<LegalIdentity>> {
    legal_identity_from_values([
        std::env::var("FOCUSNOOK_LEGAL_ADDRESS").ok(),
        std::env::var("FOCUSNOOK_LEGAL_NAME").ok(),
        std::env::var("FOCUSNOOK_SUPPORT_EMAIL").ok(),
        std::env::var("FOCUSNOOK_LEGAL_TAX_ID").ok(),
    ])
}

fn legal_identity_from_values(values: [Option<String>; 4]) -> AppResult<Option<LegalIdentity>> {
    let [address, name, support_email, tax_id] = values.map(|value| {
        value
            .map(|text| text.trim().to_string())
            .filter(|text| !text.is_empty())
    });
    if [&address, &name, &support_email, &tax_id]
        .iter()
        .all(|value| value.is_none())
    {
        return Ok(None);
    }
    match (name, support_email) {
        (Some(name), Some(support_email)) => Ok(Some(LegalIdentity {
            address,
            name,
            support_email,
            tax_id,
        })),
        _ => Err(AppError::Config(
            "public registration requires operator name and support email".to_string(),
        )),
    }
}

fn required(name: &str) -> AppResult<String> {
    std::env::var(name).map_err(|_| AppError::Config(format!("{name} is required")))
}

fn optional(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

fn decode_key(name: &str) -> AppResult<Vec<u8>> {
    let raw = required(name)?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(raw.trim())
        .map_err(|_| AppError::Config(format!("{name} must be base64")))?;
    if bytes.len() != 32 {
        return Err(AppError::Config(format!("{name} must decode to 32 bytes")));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_key_length() {
        std::env::set_var("BAD_KEY", "YWJj");
        let err = decode_key("BAD_KEY").err();
        assert!(matches!(err, Some(AppError::Config(_))));
    }

    #[test]
    fn legal_identity_can_be_disabled_for_private_sync() {
        assert!(matches!(
            legal_identity_from_values([None, None, None, None]),
            Ok(None)
        ));
    }

    #[test]
    fn legal_identity_accepts_contact_only_configuration() {
        let result = legal_identity_from_values([
            None,
            Some("ProAnimaStudio".into()),
            Some("info@proanima.net".into()),
            None,
        ]);
        assert!(matches!(result, Ok(Some(_))));
    }

    #[test]
    fn legal_identity_rejects_missing_operator_contact() {
        let result = legal_identity_from_values([Some("address".into()), None, None, None]);
        assert!(matches!(result, Err(AppError::Config(_))));
    }

    #[test]
    fn request_limit_accounts_for_base64_blob_expansion() -> AppResult<()> {
        let blob_bytes = 15 * 1024 * 1024;
        let minimum = minimum_blob_request_bytes(blob_bytes)?;
        assert_eq!(minimum, 21 * 1024 * 1024);
        assert!(validate_blob_request_limit(blob_bytes, minimum).is_ok());
        assert!(validate_blob_request_limit(blob_bytes, minimum - 1).is_err());
        Ok(())
    }
}
