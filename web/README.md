# K-O Palace Web

Standalone React + Vite client for K-O Palace package discovery and trust review.

## What it does

- Searches K-O Palace packages through `GET /api/v1/search`
- Switches between `Pandora Mode` and `Agent Mode`
- Filters by trust level, signature presence, license, and runtime compatibility
- Inspects package trust, compatibility, capabilities, and provenance in a read-only dialog
- Supports an independent light/dark theme toggle

## Prerequisites

- Node.js 20.19+
- npm

## Install

```bash
cd web
npm install
```


## Development

```bash
cd web
npm run dev
```

The dev server defaults to Vite's local host and reads `VITE_PALACE_API_URL` if set.

## Build

```bash
cd web
npm run build
```

Build output is written to `web/dist`.

## API configuration

The client uses this environment variable:

```bash
VITE_PALACE_API_URL=http://127.0.0.1:3001
```

- Production builds accept only an explicit HTTPS API origin.
- Local development may use HTTP only with `localhost`, `127.0.0.1`, or `::1`.
- If the variable is omitted, the client uses the current origin and calls `/api/v1/search`.

Copy `web/.env.example` to `web/.env` for local development if needed.

## Notes

- `Pandora Mode` focuses on Pandora-compatible genes, harnesses, skills, MCP-style connectors, and integrations.
- `Agent Mode` is an adapter and discovery view for external runtimes only. It does not run agents.
