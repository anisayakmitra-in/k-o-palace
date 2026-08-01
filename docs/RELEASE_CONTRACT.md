# Release Contract

A K-O Palace release is published only from an existing tag that is reachable from `origin/main` and is exactly `v` plus the `[package]` version in `Cargo.toml`, after the CI release gates pass.

The release workflow builds these targets:

- Linux `x86_64-unknown-linux-gnu`
- Windows `x86_64-pc-windows-msvc`
- macOS `x86_64-apple-darwin`
- macOS `aarch64-apple-darwin`

Each release uses the Rust 1.97.1 toolchain and includes packaged binaries, SHA-256 checksums, a CycloneDX SBOM, and GitHub artifact attestations for the final release files. The workflow verifies that every final archive has exactly one valid checksum sidecar and verifies every sidecar before creating attestations or publishing. Checksum sidecars contain artifact basenames so they verify from the download directory without recreating CI paths. Consumers should verify both the checksum and the attestation before installation: `sha256sum -c <archive>.sha256` and `gh attestation verify <archive> --repo anisayakmitra-in/K-O-Palace --signer-workflow anisayakmitra-in/K-O-Palace/.github/workflows/release.yml`. A green CI run alone is not a release. Platform-specific signing remains a follow-up for native installers.
