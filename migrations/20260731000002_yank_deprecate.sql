-- Add yanked and deprecated columns to packages
ALTER TABLE packages ADD COLUMN IF NOT EXISTS yanked BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE packages ADD COLUMN IF NOT EXISTS deprecated TEXT;