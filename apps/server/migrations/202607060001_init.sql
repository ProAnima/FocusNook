CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    display_name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    disabled_at TIMESTAMPTZ
);

CREATE TABLE user_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    label TEXT NOT NULL DEFAULT 'primary',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at TIMESTAMPTZ
);

CREATE TABLE devices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    platform TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    UNIQUE (user_id, device_id)
);

CREATE TABLE sync_operations (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    profile_id TEXT NOT NULL,
    operation_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    op TEXT NOT NULL,
    hlc TEXT NOT NULL,
    schema_version INTEGER NOT NULL,
    operation_digest TEXT NOT NULL,
    payload_ciphertext_enc BYTEA NOT NULL,
    payload_nonce TEXT,
    payload_key_id TEXT,
    server_nonce BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, profile_id, operation_id)
);

CREATE INDEX idx_sync_operations_pull
    ON sync_operations (user_id, profile_id, hlc, operation_id);

CREATE INDEX idx_sync_operations_device
    ON sync_operations (user_id, profile_id, device_id);

CREATE TABLE sync_blobs (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    profile_id TEXT NOT NULL,
    blob_id TEXT NOT NULL,
    content_type TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    bytes_enc BYTEA NOT NULL,
    server_nonce BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, profile_id, blob_id)
);

CREATE INDEX idx_sync_blobs_profile
    ON sync_blobs (user_id, profile_id, created_at);
