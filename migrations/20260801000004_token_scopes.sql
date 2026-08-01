-- Add explicit scopes; empty legacy values fail closed until an operator reissues the token.
ALTER TABLE api_tokens ADD COLUMN IF NOT EXISTS scopes JSONB NOT NULL DEFAULT '[]';