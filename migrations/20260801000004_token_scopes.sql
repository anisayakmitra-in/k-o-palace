-- Add explicit scopes to API tokens while preserving existing tokens as unrestricted legacy tokens.
ALTER TABLE api_tokens ADD COLUMN IF NOT EXISTS scopes JSONB NOT NULL DEFAULT '[]';