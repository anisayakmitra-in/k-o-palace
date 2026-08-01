-- Reviews are published by default and can be hidden by moderators without deletion.
ALTER TABLE reviews
    ADD COLUMN IF NOT EXISTS status VARCHAR(16) NOT NULL DEFAULT ''published'';
ALTER TABLE reviews
    ADD COLUMN IF NOT EXISTS moderated_by UUID REFERENCES publishers(id) ON DELETE SET NULL;
ALTER TABLE reviews
    ADD COLUMN IF NOT EXISTS moderation_reason TEXT;
ALTER TABLE reviews
    ADD COLUMN IF NOT EXISTS moderated_at TIMESTAMPTZ;

UPDATE reviews
SET status = ''published''
WHERE status IS NULL;

UPDATE reviews
SET moderation_reason = NULL
WHERE moderation_reason IS NOT NULL
  AND btrim(moderation_reason) = ;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = reviews_status_check
    ) THEN
        ALTER TABLE reviews
            ADD CONSTRAINT reviews_status_check
            CHECK (status IN (published, hidden));
    END IF;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = reviews_moderation_reason_check
    ) THEN
        ALTER TABLE reviews
            ADD CONSTRAINT reviews_moderation_reason_check
            CHECK (
                moderation_reason IS NULL
                OR char_length(btrim(moderation_reason)) BETWEEN 1 AND 500
            );
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_reviews_status
    ON reviews(package_id, status, created_at DESC);
