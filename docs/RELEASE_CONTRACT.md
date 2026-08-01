# Release Contract

A K-O Palace release is published only from an existing `v*` tag after the CI release gates pass.

The release workflow builds these targets:

- Linux `x86_64-unknown-linux-gnu`
- Windows `x86_64-pc-windows-msvc`
- macOS `x86_64-apple-darwin`
- macOS `aarch64-apple-darwin`

Each release includes packaged binaries, SHA-256 checksums, a CycloneDX SBOM, and GitHub artifact attestations for the final release files. These attestations provide keyless build provenance; they do not replace platform signing or publisher signature verification. A green CI run alone is not a release.
