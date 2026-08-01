ALTER TABLE download_events
    ADD COLUMN IF NOT EXISTS dedupe_key VARCHAR(128);

ALTER TABLE download_events
    ADD COLUMN IF NOT EXISTS bucket_start TIMESTAMPTZ;

CREATE UNIQUE INDEX IF NOT EXISTS idx_downloads_dedupe
    ON download_events(package_id, package_version, dedupe_key, bucket_start);
