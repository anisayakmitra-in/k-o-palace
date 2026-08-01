# K-O Palace API Contract

K-O Palace exposes a versioned HTTP API under `/api/v1`. Clients may depend on existing read routes and stable error codes. Breaking write or response changes require a new API version.

## Runtime

- `GET /health` reports process and repository health.
- `GET /ready` reports readiness for traffic.
- `GET /version` reports the service version.
- PostgreSQL is the production repository. The in-memory repository is for tests and local development.
- Supported artifact backends are `local` and `github`. Configuring another backend fails startup rather than silently changing providers.
- The server enforces request-size and timeout limits, bounded artifact verification, request IDs, tracing, and graceful shutdown.

## Authentication

Write routes require `Authorization: Bearer <token>`. Tokens are scoped and may expire. Supported scopes are `packages:read`, `packages:publish`, `packages:write`, `tokens:manage`, `moderation:write`, and `admin:write`.

Public publisher registration is disabled by default. Operators may explicitly enable it with `PALACE_ALLOW_PUBLIC_REGISTRATION=true`; public deployments should add identity verification and abuse controls before enabling it.

## Registry routes

- `GET /api/v1/packages` — list packages with pagination and filters.
- `POST /api/v1/packages` — publish an immutable package version.
- `GET /api/v1/packages/{id}` — read the latest package version.
- `GET /api/v1/packages/{id}/versions` — list package versions.
- `GET /api/v1/packages/{id}/versions/{version}` — read an exact version.
- `GET /api/v1/packages/{id}/download` — verify and redirect to a non-yanked artifact.
- `GET /api/v1/search` — search packages with bounded query input.
- `GET /api/v1/publishers` and `GET /api/v1/publishers/{name}` — read publisher profiles.
- `GET /api/v1/packages/{id}/reviews` — list reviews.

Publisher, token, trust, review, and package mutation routes are authenticated and authorization-checked.

## Errors

Errors are JSON objects with a stable `code`, human-readable `message`, and optional `details`. Clients should branch on `code`, not message text. Rate-limited responses use HTTP `429`; authentication failures use `401`; authorization failures use `403`; immutable versions use `409`.

## Deployment requirements

Production deployments should use PostgreSQL, explicit CORS origins, TLS at a trusted proxy, secret-manager supplied credentials, backups, migration verification, and a shared rate limiter before horizontal scaling.