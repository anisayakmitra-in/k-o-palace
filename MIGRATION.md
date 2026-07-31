# Pandora Client Migration Note

## What Changed

K-O Palace has been productionized. The public API (`/api/v1`) is backward-compatible with the existing read endpoints. All existing package metadata fields are preserved.

## Required Actions for Pandora Clients

### 1. No changes needed for read operations

`GET /api/v1/packages`, `GET /api/v1/packages/:id`, `GET /api/v1/packages/:id/versions`, `GET /api/v1/packages/:id/versions/:version`, `GET /api/v1/search`, `GET /api/v1/categories`, `GET /api/v1/featured`, `GET /api/v1/trending`, `GET /api/v1/newest`, `GET /api/v1/runtimes` — all work without authentication.

### 2. Authentication required for write operations

`POST /api/v1/packages`, `PUT /api/v1/packages/:id`, `DELETE /api/v1/packages/:id`, `POST /api/v1/packages/:id/reviews` now require a `Bearer` token in the `Authorization` header.

```bash
Authorization: Bearer kop_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

### 3. Trust levels can no longer be self-assigned by clients

Clients may publish packages at `Experimental` or `Community` trust level. Levels `Verified`, `Official`, `Enterprise`, and `Certified` require moderator/administrator approval and cannot be set by the publishing client. If a client publishes a package with `trust.level = "verified"`, it will be normalized to `Community` server-side.

### 4. Pagination response format

List endpoints now return:

```json
{
  "total": 150,
  "limit": 20,
  "offset": 0,
  "packages": [...]
}
```

The `total` field represents the complete filtered result count, not just the page size.

### 5. Structured error responses

Errors now return JSON with stable error codes:

```json
{
  "code": "UNAUTHORIZED",
  "message": "invalid or revoked token"
}
```

### 6. Artifact URLs must be HTTPS

Artifact URLs must use HTTPS and must be served from an allowlisted host. HTTP artifact URLs are rejected in production mode.

### 7. New endpoints

- `GET /health` — health check
- `GET /ready` — readiness check (same as health)
- `GET /version` — server version

## Backward Compatibility

All package metadata fields expected by Pandora (`id`, `name`, `version`, `kind`, `description`, `author`, `license`, `trust`, `compatibility`, `repository`, `artifact_url`, `tags`) are preserved. Additional fields are allowed and ignored by Pandora clients.

## No-Break Migration

Pandora clients using read-only operations require no changes. Clients that publish packages need to register a publisher and obtain an API token. The existing in-memory storage is preserved for development; PostgreSQL is required for production.
