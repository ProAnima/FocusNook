use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptRequest {
    pub alias: String,
    pub plaintext_base64: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptResponse {
    pub ciphertext_base64: String,
    pub iv_base64: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecryptRequest {
    pub alias: String,
    pub ciphertext_base64: String,
    pub iv_base64: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecryptResponse {
    pub plaintext_base64: String,
}
