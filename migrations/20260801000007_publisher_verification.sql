CREATE TABLE IF NOT EXISTS publisher_verifications (
    publisher_id UUID PRIMARY KEY REFERENCES publishers(id) ON DELETE CASCADE,
    verified BOOLEAN NOT NULL DEFAULT FALSE,
    verified_at TIMESTAMPTZ,
    verified_by UUID REFERENCES publishers(id) ON DELETE SET NULL,
    reason VARCHAR(500),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (NOT verified OR verified_at IS NOT NULL)
);

CREATE INDEX IF NOT EXISTS idx_publisher_verifications_verified
    ON publisher_verifications(verified);
