# KUBER Manifest Specification v1.0

## Status: Draft

## Abstract

KUBER is an open package specification for AI runtime components. It defines a standard manifest format, dependency model, trust metadata, and compatibility rules. Any runtime, tool, or platform can implement the KUBER specification. K-O Palace is the flagship registry and marketplace implementing this specification.

## Relationship to Pandora

Pandora is the flagship runtime consuming KUBER packages. However, KUBER is runtime-agnostic. Other runtimes (Goose, Cline, Continue, Aider, Claude Code, Codex, OpenCode) can consume KUBER packages via integration adapters.

---

## 1. Package Types

A KUBER package is any of:

| Type | Description |
|------|-------------|
| `gene` | Atomic executable capability |
| `domain_harness` | Domain-specific orchestration with policies, workflows, preferred genes |
| `meta_harness` | Coordination layer between harnesses |
| `source_harness` | Defines how the runtime itself operates (replacing one changes the substrate) |
| `package` | Bundle of genes + harness + policies + benchmarks + datasets + workflows |
| `provider` | LLM provider backend |
| `skill` | SKILL.md loaded capability |
| `memory_schema` | Memory schema template |
| `runtime_extension` | Runtime extension |
| `capability_pack` | Bundle of capabilities |
| `template` | Project template |
| `persona` | AI persona definition |
| `policy` | Governance policy definition |
| `benchmark` | Performance benchmark |
| `dataset` | Training/eval dataset |
| `plugin` | Dynamically loaded plugin |
| `connector` | External protocol connector (MCP, API, bridge) |
| `sdk` | SDK library |
| `distribution` | Complete runtime distribution (e.g. "Research Edition") |

---

## 2. Manifest Format

Every KUBER package contains a `kuber.toml` manifest:

```toml
[package]
id = "browser.chrome"
name = "Chrome Browser Gene"
version = "1.4.0"
author = "openpandora"
description = "Browser automation gene using Chrome DevTools Protocol"
license = "MIT"
homepage = "https://github.com/openpandora/browser-gene"
repository = "https://github.com/openpandora/browser-gene"
kind = "gene"

[package.trust]
level = "verified"          # experimental | community | verified | official | enterprise | certified
signature = "ed25519:..."   # Ed25519 signature of content hash
content_hash = "sha256:..." # SHA256 of the package tarball
publisher = "openpandora"
min_runtime_version = "0.2"

[capabilities]
provides = ["browser.open", "browser.click", "browser.extract", "browser.download"]
requires = []               # depends on capabilities, not names

[metadata]
tags = ["browser", "automation", "chrome"]
icon = "icon.png"
documentation = "README.md"
examples = ["examples/basic.rs"]

[compatibility]
runtimes = ["pandora>=0.2", "goose", "cline"]
platforms = ["linux", "macos", "windows"]
```

---

## 3. Trust Levels

| Level | Meaning |
|-------|---------|
| `experimental` | Unreviewed, no guarantees, use at own risk |
| `community` | Published by a community member, basic checks passed |
| `verified` | Publisher identity verified, signature valid, security scan passed |
| `official` | Published by the runtime's official team |
| `enterprise` | Published by an enterprise with commercial support |
| `certified` | Passed full security audit, compatibility testing, and certification |

---

## 4. Dependency Model

KUBER packages depend on **capabilities**, not package names.

```toml
[capabilities]
provides = ["filesystem.read", "filesystem.write"]
requires = ["provider.inference"]
```

The runtime resolves: "Who provides `provider.inference`?" → finds installed genes that provide it. No hardcoding.

---

## 5. Storage Backends

KUBER separates the **registry** (metadata) from **distribution** (storage).

### Registry
- Stores: package metadata, versions, manifests, trust info, reviews, downloads
- Does NOT store: package files (tarballs, binaries)

### Distribution
Package files are stored in pluggable backends:

| Backend | Use case |
|---------|----------|
| `github` | Default for community packages (GitHub Releases) |
| `gitlab` | GitLab Releases |
| `codeberg` | Codeberg Releases |
| `oci` | OCI Registry (Docker Hub, GitHub Container Registry) |
| `s3` | Amazon S3 |
| `azure` | Azure Blob Storage |
| `gcs` | Google Cloud Storage |
| `self-hosted` | Self-hosted file server |
| `local` | Local filesystem |

---

## 6. Registry API

The KUBER Registry API is a REST API that any client can implement.

### Endpoints

```
GET    /api/v1/packages                    # List packages (paginated)
GET    /api/v1/packages/:id                # Get package metadata
GET    /api/v1/packages/:id/versions       # List versions
GET    /api/v1/packages/:id/versions/:ver  # Get specific version
GET    /api/v1/search?q=...               # Search packages
GET    /api/v1/categories                  # List categories
GET    /api/v1/featured                    # Featured packages
GET    /api/v1/trending                    # Trending packages
GET    /api/v1/newest                      # Newest packages
POST   /api/v1/packages                    # Publish a package (auth required)
PUT    /api/v1/packages/:id                # Update package metadata (auth required)
DELETE /api/v1/packages/:id                # Remove package (auth required)
GET    /api/v1/packages/:id/download        # Redirect to storage backend
POST   /api/v1/packages/:id/reviews        # Add review (auth required)
GET    /api/v1/packages/:id/reviews        # List reviews
GET    /health                             # Health check
```

### Response Format

```json
{
  "id": "browser.chrome",
  "name": "Chrome Browser Gene",
  "version": "1.4.0",
  "kind": "gene",
  "description": "Browser automation gene using Chrome DevTools Protocol",
  "author": "openpandora",
  "license": "MIT",
  "trust": {
    "level": "verified",
    "signature": "ed25519:...",
    "content_hash": "sha256:...",
    "publisher": "openpandora"
  },
  "capabilities": {
    "provides": ["browser.open", "browser.click"],
    "requires": []
  },
  "downloads": 1234,
  "success_rate": 0.97,
  "compatibility": {
    "runtimes": ["pandora>=0.2", "goose", "cline"],
    "platforms": ["linux", "macos", "windows"]
  },
  "repository": "https://github.com/openpandora/browser-gene",
  "homepage": "https://github.com/openpandora/browser-gene",
  "tags": ["browser", "automation", "chrome"],
  "created_at": "2026-07-24T00:00:00Z",
  "updated_at": "2026-07-24T12:00:00Z"
}
```

---

## 7. Publishing Flow

```
Author creates package
  ↓
pandora publish (or kuber publish)
  ↓
Registry receives manifest + tarball
  ↓
Schema validation
  ↓
Signature verification
  ↓
Security scan
  ↓
Capability extraction
  ↓
Dependency resolution check
  ↓
Trust evaluation
  ↓
Store metadata in registry
  ↓
Store tarball in storage backend (GitHub Releases by default)
  ↓
Published
```

---

## 8. Installation Flow

```
pandora install browser.chrome
  ↓
Registry resolves package ID
  ↓
Check compatibility with runtime
  ↓
Resolve dependencies (capability-based)
  ↓
Download from storage backend
  ↓
Verify signature + content hash
  ↓
Policy approval (Parliament governance)
  ↓
Sandbox installation
  ↓
Register capabilities in runtime
  ↓
Ready
```

---

## 9. Categories

| Category | Package types |
|----------|---------------|
| AI Agents | Integration packages for coding agents |
| Coding Agents | Agent integration adapters |
| Genes | Atomic capabilities |
| Harnesses | Source, Meta, Domain harnesses |
| MCP Servers | Model Context Protocol servers |
| Skills | SKILL.md capabilities |
| Prompt Packs | Reusable prompt collections |
| Tools | Standalone tools |
| Plugins | Dynamically loaded plugins |
| Extensions | Runtime extensions |
| Integrations | External service connectors |
| Providers | LLM provider backends |
| Models | AI models |
| Workflows | Reusable workflows |
| Memory Packs | Memory schema templates |
| Templates | Project templates |
| Datasets | Training/eval datasets |
| Benchmarks | Performance benchmarks |
| Themes | UI themes |
| Personas | AI persona definitions |

---

## 10. Governance

The KUBER specification is open source. Anyone can implement it.

```
KUBER Specification (this document)
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

---

## License

The KUBER Manifest Specification is licensed under MIT. Anyone may implement it.
