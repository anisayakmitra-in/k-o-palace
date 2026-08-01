# K-O Palace: Current State

**Reviewed:** 2026-08-01
**Reviewed branch:** `codex/k-o-postgres-parity`

K-O Palace is a Rust registry API. Its default build uses PostgreSQL; the explicit `--no-default-features` build uses an in-memory backend for tests and local development. It is not ready to operate as a public marketplace yet.

## Works Today

- Versioned HTTP endpoints for packages, publishers, tokens, reviews, search, discovery lists, health, readiness, and version information.
- Publisher registration returns a bearer token once. Tokens are bcrypt-hashed and can be revoked through the HTTP API.
- Publishing, updating, deleting, and reviewing require bearer authentication. The API checks publisher ownership before package mutation.
- Package metadata validation covers required fields, SemVer, HTTPS URLs, basic package IDs, kinds, trust metadata, compatibility, and capabilities.
- The in-memory backend supports package versions, publisher lookup, package search, discovery lists, reviews, token management, trust-transition records, and yanking helpers.
- The publisher directory returns public publisher profiles in a deterministic name order. Moderator and administrator decisions are stored in a durable publisher-verification record and audit log.
- Artifact fetches enforce HTTPS, allowed hosts, safe resolved destinations, per-redirect validation, configured response limits, and optional SHA-256 verification. Ed25519 verification is available as a library helper. The runtime rejects unimplemented storage backends instead of aliasing them to GitHub.
- The PostgreSQL adapter persists publisher ownership, publisher verification, package trust metadata, verified artifact metadata, and audit events.

## Runtime Boundary

The default feature set selects `AppState::postgres`, runs SQLx migrations, and fails startup if the configured database is unavailable. The in-memory backend is selected only with `--no-default-features`.

## Known Gaps

### PostgreSQL parity

The current migrations and adapter persist package ownership and trust state, and the database-backed integration test covers those fields plus yanking. Broader parity coverage remains incomplete: concurrent publish, review, download, and trust-transition behavior is not covered.

### Publication and artifacts

- The publish route requires a declared digest, verifies fetched artifact bytes and any supplied signature, then transactionally persists the package, artifact metadata, signature metadata, and publication audit event.
- `fetch_and_verify` is a library helper behind the optional `reqwest` feature and is called by the publish route before the transaction begins.
- Artifact fetches enforce the configured size limit while streaming bytes to a temporary file; publish and download redirects re-fetch and verify the declared digest and signature. Signed artifacts also obey a separate in-memory verification limit.
- Both persistence backends reject an existing `(id, version)` as an immutable release. Package deletion now creates a durable yank and records an audit event; yanked packages cannot be downloaded.
- The process can seed hardcoded sample packages when configured. A public registry should not rely on a bundled catalog.

### Access and social features

- Public publisher registration is disabled by default and must be explicitly enabled with PALACE_ALLOW_PUBLIC_REGISTRATION=true. Publisher verification is moderator-gated and durable; enabled public registration still needs identity verification, invitations, and stronger anti-abuse controls.
- Tokens support explicit scopes, expiry input, UUID lookup, revocation, and last-used persistence. Legacy tokens without scopes remain unrestricted for compatibility; new tokens should request least privilege.

- Authenticated rate limits use a one-way token key. Anonymous traffic shares a bucket unless a deployment explicitly enables trusted forwarded headers; a distributed deployment still needs a shared limiter.
- Reviews enforce one review per publisher and can be published or hidden by moderators. Edit/delete workflows and reputation calculation remain out of scope.
- Discovery endpoints exist, but featured and trending use simple stored package fields. Download events are persisted and the same request context is counted once per hourly bucket; distributed fraud detection and cross-node analytics remain incomplete.
- The standalone web/ client provides discovery, Pandora/Agent mode filtering, trust filters, and theme switching. User feeds, follows, media, notifications, and social publishing remain out of scope.

### Registry ecosystem

- Capability dependency resolution is available through GET /api/v1/packages/{id}/resolve with runtime/platform filters, yanked exclusion, deterministic ranking, cycle protection, and a bounded graph walk. Version constraints, lockfiles, compatibility decisions beyond the current filters, runtime adapters, source-forge provenance, and package transfer/tombstone policy remain incomplete.
- The package identity validator accepts flat IDs and does not normalize published IDs through the namespace model in `src/identity.rs`.
- `docs/API_CONTRACT.md` publishes the current route, authentication, error, and deployment contract. A machine-generated OpenAPI document is still a follow-up.

## Verified Commands

The following commands passed on this branch after the ownership and trust parity change:

```text
cargo fmt --all -- --check
cargo check --locked --all-targets
cargo check --locked --all-targets --features postgres
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --locked --all-targets --features postgres -- -D warnings
cargo test --locked --all-targets
cargo test --locked --all-targets --features postgres
```

The PostgreSQL integration test remains conditional on KOP_TEST_DATABASE_URL; CI supplies that database. Release artifacts currently include checksums and an SBOM; cryptographic signing still requires repository secrets and key-rotation procedures.

## Next Order

1. Add concurrent PostgreSQL parity tests for publish, review, download, and trust transitions.
2. Add invitations, identity verification, and stronger public-registration abuse controls.
3. Add version constraints, lockfiles, and compatibility decisions beyond the current resolver.
4. Add distributed download fraud detection and stronger anti-manipulation analytics.
5. Add runtime adapters, package transfer policy, and a machine-generated OpenAPI document.
6. Configure release signing keys, rotation procedures, and hosted web deployment controls.
