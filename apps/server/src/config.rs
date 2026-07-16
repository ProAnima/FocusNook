use crate::error::{AppError, AppResult};
use crate::legal::LegalIdentity;
use base64::Engine;
use std::net::SocketAddr;

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
    pub legal_identity: LegalIdentity,
    pub public_base_url: String,
    pub token_pepper: Vec<u8>,
}

impl Config {
    pub fn from_env() -> AppResult<Self> {
        Ok(Self {
            admin_token: required("FOCUSNOOK_ADMIN_TOKEN")?,
            admin_web_password: required("FOCUSNOOK_WEB_SECONDARY_PASSWORD")?,
            bind_addr: optional("FOCUSNOOK_BIND_ADDR", "0.0.0.0:8080").parse()?,
            database_url: required("FOCUSNOOK_DATABASE_URL")?,
            encryption_key: decode_key("FOCUSNOOK_ENCRYPTION_KEY_B64")?,
            max_blob_bytes: optional("FOCUSNOOK_MAX_BLOB_BYTES", "15728640").parse()?,
            max_operation_payload_bytes: optional(
                "FOCUSNOOK_MAX_OPERATION_PAYLOAD_BYTES",
                "262144",
            )
            .parse()?,
            max_ops_per_exchange: optional("FOCUSNOOK_MAX_OPS_PER_EXCHANGE", "500").parse()?,
            legal_identity: LegalIdentity {
                address: required("FOCUSNOOK_LEGAL_ADDRESS")?,
                name: required("FOCUSNOOK_LEGAL_NAME")?,
                support_email: required("FOCUSNOOK_SUPPORT_EMAIL")?,
                tax_id: required("FOCUSNOOK_LEGAL_TAX_ID")?,
            },
            public_base_url: optional("FOCUSNOOK_PUBLIC_BASE_URL", "http://localhost:8080"),
            token_pepper: decode_key("FOCUSNOOK_TOKEN_PEPPER_B64")?,
        })
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
}
