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
- The publisher directory returns public publisher profiles in a deterministic name order.
- Artifact fetches enforce HTTPS, allowed hosts, safe resolved destinations, per-redirect validation, configured response limits, and optional SHA-256 verification. Ed25519 verification is available as a library helper. The runtime rejects unimplemented storage backends instead of aliasing them to GitHub.
- The PostgreSQL adapter persists publisher ownership, package trust metadata, verified artifact metadata, and audit events. Its database-backed integration test runs when `KOP_TEST_DATABASE_URL` is provided, including in CI. The binary also drains active requests on Ctrl+C and Unix terminate signals.

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

- Public publisher registration is disabled by default and must be explicitly enabled with `PALACE_ALLOW_PUBLIC_REGISTRATION=true`. Enabled registration still issues a publishing token and needs identity verification, invitations, and stronger anti-abuse controls for a public service.
- Tokens support explicit scopes, expiry input, UUID lookup, revocation, and last-used persistence. Legacy tokens without scopes remain unrestricted for compatibility; new tokens should request least privilege.

- Authenticated rate limits use a one-way token key. Anonymous traffic shares a bucket unless a deployment explicitly enables trusted forwarded headers; a distributed deployment still needs a shared limiter.
- Reviews can be created, but there is no one-review policy, edit/delete flow, moderation route, or reputation calculation.
- Discovery endpoints exist, but featured and trending use simple stored package fields. Download events and anti-manipulation rules are not implemented.
- There is no web client, user feed, follow graph, post model, media system, notification system, or moderation workflow.

### Registry ecosystem

- Dependency constraints, resolution, lockfiles, compatibility decisions, runtime adapters, source-forge provenance, and package transfer/tombstone policy are incomplete or absent from the running API.
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

The PostgreSQL integration test remains conditional on `KOP_TEST_DATABASE_URL`; CI supplies that database.

## Next Order

1. Repair and test the PostgreSQL schema and adapter as one compatibility pass.
2. Select PostgreSQL explicitly at startup and add database-backed integration tests to CI.
3. Stream artifact verification without retaining the entire artifact in memory.
4. Add scoped token lookup and per-client abuse controls.
5. Add publisher verification, review controls, and deterministic discovery rules.
6. Build the web client after the registry API has dependency resolution, install verification, and durable operational controls.
