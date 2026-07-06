CREATE TABLE sync_traffic_counters (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    inbound_bytes BIGINT NOT NULL DEFAULT 0,
    outbound_bytes BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
