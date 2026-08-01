-- Persist the publisher and verified trust metadata for every package version.
ALTER TABLE packages ADD COLUMN IF NOT EXISTS trust_level VARCHAR(32) NOT NULL DEFAULT 'community';
ALTER TABLE packages ADD COLUMN IF NOT EXISTS content_hash VARCHAR(128);
ALTER TABLE packages ADD COLUMN IF NOT EXISTS public_key TEXT;
ALTER TABLE packages ADD COLUMN IF NOT EXISTS signature TEXT;

UPDATE packages AS package
SET publisher_id = publisher.id
FROM publishers AS publisher
WHERE package.publisher_id IS NULL
  AND package.author = publisher.name;

CREATE INDEX IF NOT EXISTS idx_packages_publisher ON packages(publisher_id);