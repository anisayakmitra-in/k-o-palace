# Release Contract

A K-O Palace release is published only from an existing `v*` tag after the CI release gates pass.

The release workflow builds these targets:

- Linux `x86_64-unknown-linux-gnu`
- Windows `x86_64-pc-windows-msvc`
- macOS `x86_64-apple-darwin`
- macOS `aarch64-apple-darwin`

Each release includes a packaged binary, SHA-256 checksums, and a CycloneDX SBOM. Cryptographic artifact signing is not enabled until repository signing secrets and key rotation procedures are configured. A green CI run alone is not a release.