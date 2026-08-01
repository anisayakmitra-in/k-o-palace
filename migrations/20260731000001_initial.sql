-- K-O Palace initial schema
-- Publishers, API tokens, packages, versions, manifests, artifacts, signatures, reviews, downloads, audit events

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS publishers (
    id          UUID PRIMARY KEY,
    name        VARCHAR(64) UNIQUE NOT NULL,
    display_name VARCHAR(256) NOT NULL,
    email       VARCHAR(256),
    website     VARCHAR(512),
    role        VARCHAR(32) NOT NULL DEFAULT 'publisher',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS api_tokens (
    id           UUID PRIMARY KEY,
    publisher_id UUID NOT NULL REFERENCES publishers(id) ON DELETE CASCADE,
    token_hash   VARCHAR(128) NOT NULL,
    description  VARCHAR(256),
    name         VARCHAR(256) NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at   TIMESTAMPTZ,
    expires_at   TIMESTAMPTZ,
    last_used_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_api_tokens_publisher ON api_tokens(publisher_id);
CREATE INDEX IF NOT EXISTS idx_api_tokens_hash ON api_tokens(token_hash);

CREATE TABLE IF NOT EXISTS packages (
    id           VARCHAR(128) NOT NULL,
    name         VARCHAR(256) NOT NULL,
    version      VARCHAR(64) NOT NULL,
    kind         VARCHAR(32) NOT NULL,
    description  TEXT NOT NULL DEFAULT '',
    author       VARCHAR(256) NOT NULL,
    license      VARCHAR(128) NOT NULL,
    publisher_id UUID REFERENCES publishers(id) ON DELETE SET NULL,
    repository   VARCHAR(512),
    artifact_url VARCHAR(512),
    homepage     VARCHAR(512),
    tags         JSONB NOT NULL DEFAULT '[]',
    capabilities JSONB NOT NULL DEFAULT '{}',
    compatibility JSONB NOT NULL DEFAULT '{}',
    provenance   JSONB,
    downloads    BIGINT NOT NULL DEFAULT 0,
    success_rate REAL NOT NULL DEFAULT 0.0,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, version)
);
CREATE INDEX IF NOT EXISTS idx_packages_kind ON packages(kind);
CREATE INDEX IF NOT EXISTS idx_packages_name ON packages(name);
CREATE INDEX IF NOT EXISTS idx_packages_created ON packages(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_packages_downloads ON packages(downloads DESC);
CREATE INDEX IF NOT EXISTS idx_packages_tags ON packages USING GIN(tags);

CREATE TABLE IF NOT EXISTS manifests (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    package_id   VARCHAR(128) NOT NULL,
    package_version VARCHAR(64) NOT NULL,
    manifest     JSONB NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (package_id, package_version) REFERENCES packages(id, version) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS artifacts (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    package_id   VARCHAR(128) NOT NULL,
    package_version VARCHAR(64) NOT NULL,
    url          VARCHAR(512) NOT NULL,
    content_hash VARCHAR(128),
    content_type VARCHAR(128),
    size_bytes   BIGINT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (package_id, package_version) REFERENCES packages(id, version) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS signatures (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    package_id   VARCHAR(128) NOT NULL,
    package_version VARCHAR(64) NOT NULL,
    public_key   TEXT NOT NULL,
    signature    TEXT NOT NULL,
    verified_at  TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (package_id, package_version) REFERENCES packages(id, version) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS reviews (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    package_id   VARCHAR(128) NOT NULL,
    publisher_id UUID NOT NULL REFERENCES publishers(id) ON DELETE CASCADE,
    rating       INTEGER NOT NULL CHECK (rating >= 1 AND rating <= 5),
    comment      TEXT NOT NULL DEFAULT '',
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_reviews_package ON reviews(package_id);

CREATE TABLE IF NOT EXISTS trust_transitions (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    package_id   VARCHAR(128) NOT NULL,
    from_level   VARCHAR(32) NOT NULL,
    to_level     VARCHAR(32) NOT NULL,
    approved_by  UUID REFERENCES publishers(id),
    reason       TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_trust_package ON trust_transitions(package_id);

CREATE TABLE IF NOT EXISTS download_events (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    package_id   VARCHAR(128) NOT NULL,
    package_version VARCHAR(64) NOT NULL,
    ip_hash      VARCHAR(64),
    user_agent   VARCHAR(256),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (package_id, package_version) REFERENCES packages(id, version) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_downloads_package ON download_events(package_id);
CREATE INDEX IF NOT EXISTS idx_downloads_created ON download_events(created_at DESC);

CREATE TABLE IF NOT EXISTS audit_events (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type   VARCHAR(64) NOT NULL,
    actor_id     UUID REFERENCES publishers(id),
    target_type  VARCHAR(64),
    target_id    VARCHAR(256),
    metadata     JSONB NOT NULL DEFAULT '{}',
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_audit_created ON audit_events(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_actor ON audit_events(actor_id);
