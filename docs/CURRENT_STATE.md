# K-O Palace — Current State Audit

**Audited at:** 2026-07-31  
**HEAD:** `3fd681f`  
**Branch:** `main`

---

## What Actually Works

### Core Registry API (17 endpoints)
- `GET /health` — process alive check ✅
- `GET /ready` — same as health (does NOT check DB) ⚠️
- `GET /version` — server version ✅
- `GET /api/v1/packages` — list with pagination ✅
- `POST /api/v1/packages` — publish (auth required) ✅
- `GET /api/v1/packages/{id}` — get latest version ✅
- `PUT /api/v1/packages/{id}` — update (auth + owner) ✅
- `DELETE /api/v1/packages/{id}` — delete (auth + owner/moderator) ✅
- `GET /api/v1/packages/{id}/versions` — list versions ✅
- `GET /api/v1/packages/{id}/versions/{version}` — get specific version ✅
- `GET /api/v1/packages/{id}/download` — redirect to artifact URL ✅
- `GET /api/v1/packages/{id}/reviews` — list reviews ✅
- `POST /api/v1/packages/{id}/reviews` — add review (auth required) ✅
- `GET /api/v1/search` — ILIKE search ✅
- `GET /api/v1/categories` — derived from tags ✅
- `GET /api/v1/featured` — by success_rate + downloads ✅
- `GET /api/v1/trending` — by downloads ✅
- `GET /api/v1/newest` — by created_at ✅
- `GET /api/v1/runtimes` — derived from compatibility ✅

### Authentication
- Bearer token auth on publish/update/delete/review ✅
- bcrypt-hashed tokens at rest ✅
- Token revocation (library function, NO HTTP endpoint) ⚠️
- Publisher registration (library function, NO HTTP endpoint) ⚠️
- Role-based access (4 roles: publisher, maintainer, moderator, admin) ✅
- Publisher ownership checks ✅

### Trust System
- 6 trust levels (Experimental → Community → Verified → Official → Enterprise → Certified) ✅
- Server-enforced transitions (clients cannot self-assign above Community) ✅
- Trust transition recording with approver ✅
- Ed25519 signature verification (library function) ✅
- SHA-256 content hash verification (library function) ✅

### Validation
- Package ID format validation ✅
- SemVer validation (basic regex, NOT full SemVer crate) ⚠️
- Package kind validation ✅
- URL HTTPS validation ✅
- Trust metadata validation ✅
- Unknown security-critical field rejection ✅

### Artifact Security
- HTTPS enforcement for artifact URLs ✅
- Host allowlist validation ✅
- Default allowlist (github.com, objects.githubusercontent.com) ✅
- Content-type validation ✅
- Max artifact size (100 MB) ✅
- Redirect limit enforcement (reqwest feature) ✅
- SHA-256 content hash computation ✅

### Pagination
- Bounded limits (max 250) ✅
- Offset-based pagination ✅
- Accurate total count ✅

### Error Model
- 15 stable error codes ✅
- Structured JSON error responses ✅
- HTTP status mapping ✅

### Security Defaults
- Localhost bind (127.0.0.1:3001) ✅
- CORS from configured origins ✅
- Request body limits (16 MB) ✅
- Request timeouts (30s) ✅
- Structured tracing ✅
- No token logging (redact_token) ✅
- No unwrap() in startup ✅

### In-Memory Backend
- Full repository implementation (28 methods) ✅
- All tests pass against in-memory ✅
- Suitable for tests and local dev ✅

### PostgreSQL Backend
- Compiles cleanly with `--features postgres` ✅
- 10-table schema with 13 indexes ✅
- pgcrypto extension for gen_random_uuid() ✅
- NOT integration-tested ⚠️
- Migration CI fails (non-blocking) ⚠️

### CI Pipeline
- fmt + check + clippy + test: ✅ green
- Build release: ✅ green (2.03 MB artifact)
- Migration verify: ❌ failure (continue-on-error)
- SBOM generation: ❌ failure (continue-on-error)

### Tests (30 passing)
- 5 API integration tests ✅
- 6 artifact host validation tests ✅
- 7 auth/authorization tests ✅
- 6 trust/signature tests ✅
- 6 validation tests ✅

---

## What Is Scaffolded (type exists, not fully wired)

| Area | Status |
|------|--------|
| `VersionInfo` | Struct exists, `list_versions` works, but no dependency metadata |
| `Manifest` / `KuberManifest` | Parsed and validated, but not stored separately |
| `AuditEvent` | Struct exists, `record_audit_event` in repo, but NOT called from routes |
| `TrustTransition` | Struct exists, `record_trust_transition` in repo, `transition_trust` works, but NO HTTP endpoint |
| `ArtifactInfo` | Struct exists, `fetch_and_verify` exists, but NOT called from publish flow |
| `PackageKind` | 19 variants exist, but no extensibility mechanism (can't add new kinds without code change) |

---

## What Is Incomplete

| Area | Gap |
|------|-----|
| Publisher HTTP endpoints | No `POST /api/v1/publishers`, `GET /api/v1/publishers/{name}` |
| Token HTTP endpoints | No `POST /api/v1/tokens`, `GET /api/v1/tokens`, `DELETE /api/v1/tokens/{id}` |
| Token scopes | No scope field on `ApiToken` — all tokens have all permissions |
| Token expiration | Field exists (`expires_at`) but NOT checked during auth |
| Token last-used | No field for `last_used_at` — tracking not implemented |
| Rate limiting | Config values exist but NO middleware enforcement |
| Dependency model | No dependency fields, no resolver, no lock file |
| Namespace model | No `@publisher/package` format — flat ID space |
| Package immutability | No yank/unyank/tombstone/deprecate |
| Provenance | No commit SHA, tag, forge identity, source repository metadata |
| Forge adapter | No abstraction for GitHub/GitLab/Codeberg/Forgejo |
| Compatibility query | No "can runtime X install package Y" endpoint |
| Adapter model | No runtime adapter concept |
| Search V2 | ILIKE only, no full-text search, no ranking signals |
| Discovery algorithms | trending = downloads DESC, featured = success_rate DESC — simplistic |
| Review constraints | No one-review-per-publisher check, no update/delete review |
| Key lifecycle | No key registration, rotation, revocation, fingerprint |
| Signing payload | No canonical signing format defined |
| OpenAPI spec | None |
| Docker | None |
| Request IDs | Not generated or logged |
| Health vs Readiness | `/ready` doesn't check DB connectivity |
| Observability metrics | No metrics endpoint or instrumentation |

---

## What Is Feature-Gated

| Feature | Flag | Status |
|---------|------|--------|
| PostgreSQL backend | `postgres` | Compiles, not integration-tested |
| Artifact fetching | `reqwest` | Compiles, `fetch_with_redirect_limit` works, not called from publish |
| In-memory backend | (default) | Full support, all tests use this |

---

## What Requires PostgreSQL

- Production persistence
- CI migration verification
- Full-text search (future)
- Concurrent write safety
- Transaction boundaries for publish

---

## What Requires Artifact Fetching

- Content hash verification during publish
- Signature verification against actual artifact bytes
- Artifact metadata extraction (size, content-type)
- Streaming downloads with redirect enforcement

---

## What Is NOT Yet Marketplace-Ready

1. **No namespace model** — flat package IDs allow collision and impersonation
2. **No dependency resolution** — packages can't declare or resolve dependencies
3. **No publisher/token management endpoints** — can't register or manage tokens via API
4. **No rate limiting** — search/download/publish are unprotected
5. **No package immutability** — published versions can be silently replaced
6. **No provenance** — no Git forge metadata, commit SHAs, or source verification
7. **No compatibility query** — can't answer "can runtime X install package Y"
8. **No OpenAPI spec** — no machine-readable API contract
9. **No Docker** — can't self-host with `docker compose up`
10. **No SBOM** — supply chain verification missing
11. **No request IDs** — errors can't be traced
12. **Postgres not tested** — backend compiles but has zero integration test coverage
13. **No key lifecycle** — signing keys can't be registered, rotated, or revoked
14. **No adapter model** — packages can't declare runtime-specific installation
15. **No private/unlisted packages** — all packages are public
