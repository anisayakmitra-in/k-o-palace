# Release Contract

A K-O Palace release is published only from an existing `v*` tag after the CI release gates pass.

The release workflow builds these targets:

- Linux `x86_64-unknown-linux-gnu`
- Windows `x86_64-pc-windows-msvc`
- macOS `x86_64-apple-darwin`
- macOS `aarch64-apple-darwin`

Each release includes packaged binaries, SHA-256 checksums, a CycloneDX SBOM, and GitHub artifact attestations for the final release files. Consumers should verify both the checksum and the attestation before installation: `sha256sum -c <archive>.sha256` and `gh attestation verify <archive> --repo anisayakmitra-in/K-O-Palace --signer-workflow anisayakmitra-in/K-O-Palace/.github/workflows/release.yml`. A green CI run alone is not a release. Platform-specific signing remains a follow-up for native installers.
