-- Persist the publisher and verified trust metadata for every package version.
ALTER TABLE packages ADD COLUMN IF NOT EXISTS trust_level VARCHAR(32) NOT NULL DEFAULT 'community';
ALTER TABLE packages ADD COLUMN IF NOT EXISTS content_hash VARCHAR(128);
ALTER TABLE packages ADD COLUMN IF NOT EXISTS public_key TEXT;
ALTER TABLE packages ADD COLUMN IF NOT EXISTS signature TEXT;

-- Do not infer ownership from mutable package.author text. Legacy rows remain unowned until an operator maps them to a durable publisher ID.

CREATE INDEX IF NOT EXISTS idx_packages_publisher ON packages(publisher_id);