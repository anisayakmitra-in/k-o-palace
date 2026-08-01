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
- Artifact fetches enforce HTTPS, allowed hosts, safe resolved destinations, per-redirect validation, configured response limits, and optional SHA-256 verification. Ed25519 verification is available as a library helper.
- The PostgreSQL adapter has a database-backed integration test. It runs when `KOP_TEST_DATABASE_URL` is provided, including in CI.

## Runtime Boundary

The default feature set selects `AppState::postgres`, runs SQLx migrations, and fails startup if the configured database is unavailable. The in-memory backend is selected only with `--no-default-features`.

## Known Gaps

### PostgreSQL parity

The current migration and adapter pass the database-backed core-record integration test. Broader parity coverage remains incomplete: package ownership is still represented through `author`uthor, and concurrent publish, review, download, and trust-transition behavior is not covered.

### Publication and artifacts

- The publish route requires a declared digest and verifies fetched artifact bytes and any supplied signature before the package write, but does not yet persist artifact metadata transactionally.
- `fetch_and_verify` is a library helper behind the optional `reqwest` feature. It is not part of the publish transaction.
- Artifact fetches buffer the response before enforcing the size limit. Redirect targets are not revalidated against the artifact-host policy.
- Both persistence backends reject an existing `(id, version)` as an immutable release.
- The process can seed hardcoded sample packages when configured. A public registry should not rely on a bundled catalog.

### Access and social features

- Publisher registration is unauthenticated and immediately issues a publishing token. There is no identity-verification, invitation, or anti-abuse flow.
- Tokens have no scopes, expiry input, last-used update, or separate administrative management policy.

- Rate limits use one static key per endpoint. They are not per user, IP address, or trusted proxy identity.
- Reviews can be created, but there is no one-review policy, edit/delete flow, moderation route, or reputation calculation.
- Discovery endpoints exist, but featured and trending use simple stored package fields. Download events and anti-manipulation rules are not implemented.
- There is no web client, user feed, follow graph, post model, media system, notification system, or moderation workflow.

### Registry ecosystem

- Dependency constraints, resolution, lockfiles, compatibility decisions, runtime adapters, source-forge provenance, and package transfer/tombstone policy are incomplete or absent from the running API.
- The package identity validator accepts flat IDs and does not normalize published IDs through the namespace model in `src/identity.rs`.
- There is no OpenAPI document or published API compatibility policy.

## Verified Commands

The following commands passed on this branch after the publisher-directory change:

```text
cargo fmt --all -- --check
cargo check --locked --all-targets
cargo check --locked --all-targets --features postgres
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --locked --all-targets --features postgres -- -D warnings
cargo test --locked --all-targets
cargo test --locked --all-targets --features postgres
```

They do not replace a PostgreSQL migration test or an artifact-fetch integration test.

## Next Order

1. Repair and test the PostgreSQL schema and adapter as one compatibility pass.
2. Select PostgreSQL explicitly at startup and add database-backed integration tests to CI.
3. Make publication transactional: authorize, fetch, bound, verify, persist metadata, then publish.
4. Enforce immutable version records and finish namespace ownership.
5. Add publisher portfolio queries, review controls, and deterministic discovery rules.
6. Build the web client after the registry API has durable publisher and package ownership.
