# K-O Palace

> Open AI Runtime Registry — the sovereign ecosystem for discovering, validating, signing, versioning, evolving, and distributing AI runtime components.

K-O Palace is a runtime-agnostic AI package registry that implements the K-O Palace manifest specification. It provides secure package publishing, trust-level verification, Ed25519 signature validation, content-hash verification, and artifact storage with allowlist enforcement.

## Current Functionality

- **Versioned registry API** under /api/v1 for packages, publishers, reviews, trust, search, and dependency resolution
- **Manifest validation** for `palace.toml` (package ID, semver, kind, author, license, compatibility, capabilities, URLs, trust metadata)
- **Authentication** with hashed API tokens, publisher ownership, and role-based access (publisher, maintainer, moderator, administrator)
- **Trust levels** with explicit server-enforced transitions, backed by publisher verification for identified publishers
- **Ed25519 signature verification** and SHA-256 content-hash verification
- **Artifact delivery** from local filesystem and GitHub Release sources, with HTTPS enforcement, host allowlists, bounded streaming to temporary files, redirect limits, and checksum verification
- **Capability dependency resolution** with runtime/platform filtering, yanked-package exclusion, deterministic candidate ranking, and bounded graph traversal
- **Search** with ranked relevance scoring
- **Structured errors** with stable error codes
- **CORS** configured from explicit origins (not permissive)
- **Request body limits** and **request timeouts**
- **Structured tracing** via `tracing` / `tracing-subscriber`
- **In-memory backend** for tests; **PostgreSQL backend** via SQLx for production

## Production Functionality

- PostgreSQL persistence with SQLx migrations (10 tables: publishers, api_tokens, packages, manifests, artifacts, signatures, reviews, trust_transitions, download_events, audit_events)
- Token revocation and constant-time bcrypt comparison
- Immutable audit events for all trust transitions and critical operations
- Rate limiting for publish, search, download, review, and auth endpoints
- HTTPS enforcement for artifact URLs in production
- No client-self-assigned trust levels above Community
- Secure defaults: localhost bind, configured CORS, body limits, timeouts

## Future Roadmap

- OCI artifact registry adapter
- S3 / Azure Blob / GCS storage adapters
- GitLab and Codeberg release metadata backends
- WebHOOK-based package update notifications
- Semantic version constraints and lockfiles for dependency resolution
- Package signing key rotation workflow
- Federated registry sync (mirror mode)
- SBOM generation per package version
- Search index (PostgreSQL full-text or Meilisearch)

## Container Deployment

For a local PostgreSQL-backed deployment:

```bash
docker compose up --build
```

Change the example database password before exposing the service. Public deployments should provide secrets through the platform secret manager, terminate TLS at a trusted proxy, set explicit CORS origins, and configure backups for the PostgreSQL volume.
## Web Client

The standalone React and Vite client provides discovery and trust review without executing packages.

    cd web
    npm install
    npm run dev

Set VITE_PALACE_API_URL to point the client at a running Palace API. The web client is a discovery surface for Pandora-compatible packages and external-agent adapters; installation and execution remain explicit client actions.

## Local Development

### Prerequisites

- Rust 1.75+ (`rustup`)
- PostgreSQL 14+ (for production mode)
- Node.js 20.19+ and npm (for the web client)

### Quick Start (PostgreSQL)

```bash
DATABASE_URL=postgres://kopalace:kopalace@localhost:5432/kopalace cargo run
```

The default build uses PostgreSQL and runs migrations at startup. The server binds to `127.0.0.1:3001` by default.

For local API experiments without durable storage, use the explicit development build:

```bash
cargo run --no-default-features
```
### Environment Variables

| Variable | Default | Description |
|---|---|---|
| `PALACE_BIND` | `127.0.0.1:3001` | Server bind address |
| `PALACE_PUBLIC_URL` | `http://127.0.0.1:3001` | Public URL for downloads |
| `DATABASE_URL` | required for PostgreSQL | PostgreSQL connection string; no credentialed default is used |
| `PALACE_DB_MAX_CONNECTIONS` | `10` | PostgreSQL pool maximum |
| `PALACE_CORS_ORIGINS` | (empty) | Comma-separated allowed origins |
| `PALACE_SEED_SAMPLES` | `false` | Seed sample packages on startup |
| `PALACE_RATE_LIMIT_PUBLISH_PER_MINUTE` | `10` | Publish requests per minute per limiter key |
| `PALACE_RATE_LIMIT_SEARCH_PER_MINUTE` | `120` | Search requests per minute per limiter key |
| `PALACE_RATE_LIMIT_DOWNLOAD_PER_MINUTE` | `240` | Download requests per minute per limiter key |
| `PALACE_RATE_LIMIT_AUTH_PER_MINUTE` | `10` | Authentication and registration requests per minute per limiter key |
| `PALACE_RATE_LIMIT_RESOLVE_PER_MINUTE` | `60` | Dependency resolution requests per minute per limiter key |
| `PALACE_REQUIRE_HTTPS` | `true` | Require an HTTPS public URL when binding beyond localhost |

## PostgreSQL Setup

```bash
createdb kopalace
psql kopalace -c "CREATE USER kopalace WITH PASSWORD 'kopalace';"
psql kopalace -c "GRANT ALL ON DATABASE kopalace TO kopalace;"

# Run migrations
DATABASE_URL=postgres://kopalace:kopalace@localhost:5432/kopalace cargo run
```

### Migrations

Migrations are in `migrations/` and managed by SQLx:

```bash
cargo install sqlx-cli --no-default-features --features postgres,rustls
sqlx migrate run --source migrations
```

## Publisher Registration

```bash
# Register a new publisher (returns API token)
curl -X POST http://127.0.0.1:3001/api/v1/publishers \
  -H "Content-Type: application/json" \
  -d '{"name": "myorg", "display_name": "My Organization"}'
```

Store the returned token securely. It is shown only once.

## Token Creation and Revocation

Tokens are bcrypt-hashed at rest. The plaintext token is returned only at creation time.

```bash
# Revoke a token
curl -X DELETE http://127.0.0.1:3001/api/v1/tokens/{token_id} \
  -H "Authorization: Bearer kop_..."
```

## Package Publishing

```bash
curl -X POST http://127.0.0.1:3001/api/v1/packages \
  -H "Authorization: Bearer kop_..." \
  -H "Content-Type: application/json" \
  -d @package.json
```

Where `package.json` contains the package metadata with required fields: `id`, `name`, `version`, `kind`, `description`, `author`, `license`, `trust`, `compatibility`, `repository`, `artifact_url`, `tags`.

## Signature Creation

K-O Palace verifies Ed25519 signatures server-side. To sign a package:

```bash
# Generate Ed25519 keypair
openssl genpkey -algorithm Ed25519 -out private_key.pem
openssl pkey -in private_key.pem -pubout -out public_key.pem

# Sign the artifact content
 openssl dgst -sha256 -sign private_key.pem artifact.tar.gz | base64 > signature.b64

# Publish with signature
curl -X POST http://127.0.0.1:3001/api/v1/packages \
  -H "Authorization: Bearer kop_..." \
  -H "Content-Type: application/json" \
  -d '{"id": "...", "trust": {"level": "community", "signature": "...", "public_key": "...", "content_hash": "..."}, ...}'
```

## Artifact Hosting

Artifacts must be served over HTTPS from an allowlisted host. Default allowed hosts:
- `github.com`
- `objects.githubusercontent.com`

Configure additional hosts via `PALACE_ALLOWED_HOSTS` environment variable. Other deployment settings include `PALACE_STORAGE_BACKEND` (`local` or `github`; other values fail startup), `PALACE_ALLOW_PUBLIC_REGISTRATION` (disabled by default), `PALACE_STORAGE_LOCAL_PATH`, `PALACE_MAX_ARTIFACT_SIZE_BYTES`, `PALACE_MAX_SIGNED_ARTIFACT_SIZE_BYTES`, `PALACE_MAX_BODY_BYTES`, `PALACE_REQUEST_TIMEOUT_SECS`, and `PALACE_TRUST_PROXY_HEADERS`. Forwarded headers must only be enabled behind a proxy that overwrites them.

API tokens can request `packages:read`, `packages:publish`, `packages:write`, `tokens:manage`, `moderation:write`, or `admin:write` scopes and may include an `expires_at` timestamp. New tokens are addressable by their token ID; older tokens continue through the compatibility verifier.

## Trust Review

Trust levels above `Community` (Verified, Official, Enterprise, Certified) require moderator or administrator approval. Clients cannot self-assign these levels. Each transition is recorded with approver identity, timestamp, and reason.

## Self-Hosting

```bash
# Build release binary
cargo build --release

# Run with PostgreSQL
DATABASE_URL=postgres://user:pass@localhost:5432/kopalace \
  ./target/release/k-o-palace
```

For public deployment, set:
- `PALACE_BIND=0.0.0.0:3001`
- `PALACE_PUBLIC_URL=https://registry.example.com`
- `PALACE_CORS_ORIGINS=https://app.example.com`

## Pandora CLI Integration

K-O Palace is designed to be runtime-agnostic and compatible with Pandora's package metadata format. Pandora clients can query the registry using:

```bash
pandora palace search <query>
pandora palace install <package-id>
pandora palace list
```

The API returns package metadata with these fields (compatible with Pandora's expectations):
- `id`, `name`, `version`, `kind`, `description`, `author`, `license`
- `trust` (level, signature, public_key, content_hash, publisher)
- `compatibility` (runtimes,.arch)
- `repository`, `artifact_url`, `tags`

Additional fields are allowed and ignored by Pandora clients.

## API Compatibility

All endpoints are versioned under `/api/v1`. Breaking changes require a new API version. The existing read endpoints (`GET`) are preserved. Write endpoints (`POST`, `PUT`, `DELETE`) require authentication.

## Security Model

- **Authentication**: Bearer token (bcrypt-hashed at rest)
- **Authorization**: Role-based (publisher, maintainer, moderator, administrator)
- **Trust levels** with explicit server-enforced transitions, backed by publisher verification for identified publishers
- **Signatures**: Ed25519 verified server-side
- **Content hash**: SHA-256 verified against uploaded artifact
- **Artifacts**: HTTPS-only, host allowlisted, redirect-limited, size-limited, content-type validated, streamed as attachments with nosniff
- **CORS**: Configured origins only, never permissive
- **Rate limits**: Publish (10/min), Search (120/min), Download (240/min), Auth (10/min)
- **Request limits**: 16 MB body, 30 second timeout
- **Audit**: Immutable audit events for all trust transitions and critical operations
- **No logging** of tokens, private keys, or package secrets

## License

Apache-2.0
