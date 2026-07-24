# K-O Palace

> Open AI Runtime Registry — the sovereign ecosystem for discovering, validating, signing, versioning, evolving, and distributing AI runtime components.

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache 2.0-yellow.svg)](https://opensource.org/licenses/Apache 2.0)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)

## What is K-O Palace?

K-O Palace is an **open marketplace and registry for AI development tools**. It's not just Pandora's package manager — it's a runtime-agnostic ecosystem where any AI runtime, coding agent, or tool can publish and consume packages.

Think of it as:
- **crates.io** (packages) +
- **Hugging Face** (AI artifacts) +
- **Docker Hub** (deployable units) +
- **VS Code Marketplace** (extensions) +
- **MCP Registries** (protocol servers)

...adapted for AI runtimes.

## Pandora is the flagship runtime

Pandora is the first runtime to consume KUBER packages. But K-O Palace is runtime-agnostic. Other runtimes (Goose, Cline, Continue, Aider, Claude Code, Codex, OpenCode) can consume packages via integration adapters.

## What can be published?

Everything an AI runtime needs:

| Category | Examples |
|----------|----------|
| **Genes** | Atomic capabilities (browser, shell, filesystem, git) |
| **Harnesses** | Source, Meta, Domain orchestration layers |
| **MCP Servers** | Model Context Protocol servers |
| **Skills** | SKILL.md loaded capabilities |
| **Providers** | LLM provider backends (Ollama, OpenAI, Anthropic) |
| **Coding Agents** | Integration packages for coding agents |
| **Workflows** | Reusable multi-step execution plans |
| **Memory Packs** | Memory schema templates |
| **Templates** | Project templates |
| **Personas** | AI persona definitions |
| **Policies** | Governance policy definitions |
| **Benchmarks** | Performance benchmarks |
| **Datasets** | Training/eval datasets |
| **Plugins** | Dynamically loaded plugins |
| **Connectors** | External protocol connectors |
| **Distributions** | Complete runtime distributions |

## Architecture

```
                 K-O PALACE

          ┌────────────────────┐
          │     Backend API    │
          │     (Rust/Axum)    │
          └─────────┬──────────┘
                    │
      ┌─────────────┼─────────────┐
      │             │             │
      ▼             ▼             ▼
  Web App      Tauri App     Pandora CLI
 (Next.js)    (Desktop)    (install/publish)
```

One backend. Three ways to use it.

## KUBER Manifest Specification

The [KUBER Manifest Specification](specs/MANIFEST_SPEC.md) is an open standard. Anyone can implement it.

```
KUBER Specification
  │
  ├── Manifest Spec
  ├── Registry API
  ├── Package Format
  ├── Trust Metadata
  └── Compatibility Rules

          │

  Implementations

  ├── K-O Palace (official registry + marketplace)
  ├── Pandora (flagship runtime consumer)
  ├── Future Runtime A
  ├── Future Runtime B
  └── Community Tools
```

## Storage

K-O Palace separates the **registry** (metadata) from **distribution** (storage).

- **Registry**: Stores package metadata, versions, manifests, trust info, reviews
- **Distribution**: Package files stored in pluggable backends (GitHub Releases by default)

```
GitHub Releases (default) → GitLab → OCI → S3 → Self-hosted
```

## Trust Levels

| Level | Meaning |
|-------|---------|
| Experimental | Unreviewed, use at own risk |
| Community | Published by community member, basic checks |
| Verified | Publisher verified, signature valid, security scan passed |
| Official | Published by the runtime's official team |
| Enterprise | Published by enterprise with commercial support |
| Certified | Full security audit + compatibility testing + certification |

## Quick Start

```bash
# Clone
git clone https://github.com/anisayakmitra-in/k-o-palace.git
cd k-o-palace

# Build
cargo build --release

# Run
./target/release/k-o-palace

# API
curl http://localhost:3001/api/v1/packages
curl http://localhost:3001/api/v1/search?q=browser
curl http://localhost:3001/api/v1/featured
curl http://localhost:3001/api/v1/trending
```

## API Endpoints

```
GET    /api/v1/packages                    List packages (paginated)
GET    /api/v1/packages/:id                Get package metadata
GET    /api/v1/search?q=...               Search packages
GET    /api/v1/categories                  List categories
GET    /api/v1/featured                    Featured packages
GET    /api/v1/trending                    Trending packages
GET    /api/v1/newest                      Newest packages
POST   /api/v1/packages                    Publish a package
GET    /health                             Health check
```

## Roadmap

### Phase 1 (Current)
- [x] Rust backend (Axum)
- [x] KUBER Manifest Specification
- [x] In-memory store with sample packages
- [x] REST API: list, search, featured, trending, newest
- [ ] PostgreSQL persistence
- [ ] GitHub Releases storage backend
- [ ] CLI integration

### Phase 2
- [ ] Next.js web marketplace
- [ ] Full-text search (Meilisearch)
- [ ] Package pages with reviews
- [ ] User profiles
- [ ] Publisher verification

### Phase 3
- [ ] Tauri desktop app
- [ ] One-click install/update
- [ ] Local Pandora management
- [ ] Provider configuration

### Phase 4
- [ ] Commercial features
- [ ] Private registries
- [ ] Organizations
- [ ] Paid packages
- [ ] Enterprise support

## License

Apache 2.0 — The KUBER Manifest Specification is open source. Anyone may implement it.
