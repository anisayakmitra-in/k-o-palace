# K-O Palace: Current State

**Reviewed:** 2026-08-01
**Reviewed commit:** `d9ba92f`

K-O Palace is a Rust registry API with an in-memory runtime backend. It has a useful package and publisher model, but it is not ready to operate as a public marketplace yet.

## Works Today

- Versioned HTTP endpoints for packages, publishers, tokens, reviews, search, discovery lists, health, readiness, and version information.
- Publisher registration returns a bearer token once. Tokens are bcrypt-hashed and can be revoked through the HTTP API.
- Publishing, updating, deleting, and reviewing require bearer authentication. The API checks publisher ownership before package mutation.
- Package metadata validation covers required fields, SemVer, HTTPS URLs, basic package IDs, kinds, trust metadata, compatibility, and capabilities.
- The in-memory backend supports package versions, publisher lookup, package search, discovery lists, reviews, token management, trust-transition records, and yanking helpers.
- The publisher directory returns public publisher profiles in a deterministic name order.
- URL validation restricts artifact URLs to HTTPS and the configured allowlist. Hash and Ed25519 verification helpers exist as library functions.
- The default and `postgres` feature builds compile. The test suite currently exercises the in-memory backend in both configurations.

## Runtime Boundary

`src/main.rs` always calls `AppState::in_memory`. A normal K-O Palace process therefore does not select PostgreSQL, even when compiled with the `postgres` feature.

The repository includes a PostgreSQL adapter and SQLx migrations, but the automated tests do not create a PostgreSQL database or execute that adapter. A successful `cargo test --features postgres` proves compilation only; the test app still uses `AppState::in_memory`.

## Known Gaps

### PostgreSQL parity

The current migrations and SQL adapter are not aligned:

- `api_tokens` is migrated with `description` and `last_used_at`, while the adapter queries `name` and `expires_at`.
- `packages` is migrated without `provenance`, while the adapter reads and writes that column.
- `reviews` is migrated with `publisher_id`, while the adapter queries `reviewer_id`.
- `audit_events` is migrated with `target_type`, `target_id`, and `metadata`, while the adapter queries `package_id` and `details`.
- The `reviews` and `trust_transitions` foreign keys reference `packages(id)`, but the package key is `(id, version)`. Those references need a valid unique target or a composite foreign key.
- The package table has `publisher_id`, but the adapter does not write or read it. Package responses reconstruct the publisher from the user-supplied `author` field.

These discrepancies require migration repair and real PostgreSQL integration tests before PostgreSQL can be advertised as usable.

### Publication and artifacts

- The publish route validates package metadata but does not fetch the artifact, verify its supplied digest, verify its signature, or persist artifact metadata.
- `fetch_and_verify` is a library helper behind the optional `reqwest` feature. It is not part of the publish transaction.
- Artifact fetches buffer the response before enforcing the size limit. Redirect targets are not revalidated against the artifact-host policy.
- Package immutability differs by backend: the memory backend rejects an existing `(id, version)`, while the PostgreSQL adapter updates it on conflict.
- The process can seed hardcoded sample packages when configured. A public registry should not rely on a bundled catalog.

### Access and social features

- Publisher registration is unauthenticated and immediately issues a publishing token. There is no identity-verification, invitation, or anti-abuse flow.
- Tokens have no scopes, expiry input, last-used update, or separate administrative management policy.
- Publisher responses include the optional email field despite being described as public responses.
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
