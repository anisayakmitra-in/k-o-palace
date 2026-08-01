//! Manifest and metadata validation.

use crate::{
    error::{PalaceError, PalaceErrorCode, PalaceResult},
    models::{
        CapabilityInfo, CompatibilityInfo, ManifestTrust, Package, PackageKind, PalaceManifest,
        TrustInfo, TrustLevel,
    },
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use regex::Regex;
use semver::Version;
use std::collections::HashSet;
use url::Url;

/// Validate a `palace.toml` manifest against the documented specification.
pub fn validate_manifest(manifest: &PalaceManifest) -> PalaceResult<()> {
    let mut errors = Vec::new();

    validate_package_id(&manifest.package.id, &mut errors);
    validate_semver(&manifest.package.version, &mut errors);
    validate_package_kind(&manifest.package.kind, &mut errors);
    validate_non_empty(&manifest.package.name, "package.name", &mut errors);
    validate_non_empty(
        &manifest.package.description,
        "package.description",
        &mut errors,
    );
    validate_non_empty(&manifest.package.author, "package.author", &mut errors);
    validate_non_empty(&manifest.package.license, "package.license", &mut errors);
    validate_capabilities(&manifest.capabilities, &mut errors);
    validate_compatibility(&manifest.compatibility, &mut errors);
    validate_urls(manifest, &mut errors);
    validate_trust_metadata(&manifest.package.trust, &mut errors);
    validate_metadata(&manifest.metadata, &mut errors);
    reject_unknown_security_fields(manifest, &mut errors);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(PalaceError::new(
            PalaceErrorCode::InvalidManifest,
            "manifest validation failed",
        )
        .with_details(serde_json::json!({"errors": errors })))
    }
}

/// Validate a package submitted via API before it is stored.
pub fn validate_package(pkg: &Package) -> PalaceResult<()> {
    let mut errors = Vec::new();
    validate_package_id(&pkg.id, &mut errors);
    validate_semver(&pkg.version, &mut errors);
    validate_non_empty(&pkg.name, "name", &mut errors);
    validate_non_empty(&pkg.description, "description", &mut errors);
    validate_non_empty(&pkg.author, "author", &mut errors);
    validate_non_empty(&pkg.license, "license", &mut errors);
    validate_capabilities(&pkg.capabilities, &mut errors);
    validate_compatibility(&pkg.compatibility, &mut errors);
    validate_package_trust(&pkg.trust, &mut errors);
    if let Some(url) = &pkg.repository {
        if !is_valid_https_url(url) {
            errors.push(("repository", format!("invalid HTTPS URL: {url}")));
        }
    }
    if let Some(url) = &pkg.artifact_url {
        if !is_valid_https_url(url) {
            errors.push(("artifact_url", format!("invalid HTTPS URL: {url}")));
        }
    }
    if let Some(url) = &pkg.homepage {
        if !is_valid_https_url(url) {
            errors.push(("homepage", format!("invalid HTTPS URL: {url}")));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(PalaceError::new(
            PalaceErrorCode::ValidationFailed,
            "package validation failed",
        )
        .with_details(serde_json::json!({ "errors": errors })))
    }
}

fn validate_package_id(id: &str, errors: &mut Vec<(&'static str, String)>) {
    if id.is_empty() {
        errors.push(("package.id", "package ID is required".into()));
        return;
    }
    if id.len() > 128 {
        errors.push(("package.id", "package ID must be <= 128 characters".into()));
    }
    let re = Regex::new(r"^[a-zA-Z0-9][a-zA-Z0-9._-]*(/[a-zA-Z0-9][a-zA-Z0-9._-]*)*$").unwrap();
    if !re.is_match(id) {
        errors.push((
            "package.id",
            "package ID must be alphanumeric with dots, dashes, underscores, and optional namespace".into(),
        ));
    }
}

fn validate_semver(version: &str, errors: &mut Vec<(&'static str, String)>) {
    if Version::parse(version).is_err() {
        errors.push((
            "package.version",
            format!("'{}' is not a valid semantic version", version),
        ));
    }
}

fn validate_package_kind(kind: &str, errors: &mut Vec<(&'static str, String)>) {
    if PackageKind::parse(kind).is_none() {
        errors.push(("package.kind", format!("unknown package kind: {kind}")));
    }
}

fn validate_non_empty(value: &str, field: &'static str, errors: &mut Vec<(&'static str, String)>) {
    if value.trim().is_empty() {
        errors.push((field, format!("{field} cannot be empty")));
    }
    if value.len() > 4096 {
        errors.push((field, format!("{field} must be <= 4096 characters")));
    }
}

fn validate_capabilities(cap: &CapabilityInfo, errors: &mut Vec<(&'static str, String)>) {
    for (i, c) in cap.provides.iter().enumerate() {
        if c.trim().is_empty() {
            errors.push((
                "capabilities.provides",
                format!("empty capability at index {i}"),
            ));
        }
    }
    for (i, c) in cap.requires.iter().enumerate() {
        if c.trim().is_empty() {
            errors.push((
                "capabilities.requires",
                format!("empty capability at index {i}"),
            ));
        }
    }
}

fn validate_compatibility(comp: &CompatibilityInfo, errors: &mut Vec<(&'static str, String)>) {
    for (i, r) in comp.runtimes.iter().enumerate() {
        if r.trim().is_empty() {
            errors.push((
                "compatibility.runtimes",
                format!("empty runtime at index {i}"),
            ));
        }
    }
    for (i, p) in comp.platforms.iter().enumerate() {
        if p.trim().is_empty() {
            errors.push((
                "compatibility.platforms",
                format!("empty platform at index {i}"),
            ));
        }
    }
}

fn validate_urls(manifest: &PalaceManifest, errors: &mut Vec<(&'static str, String)>) {
    if let Some(url) = &manifest.package.homepage {
        if !is_valid_https_url(url) {
            errors.push(("package.homepage", format!("invalid HTTPS URL: {url}")));
        }
    }
    if let Some(url) = &manifest.package.repository {
        if !is_valid_https_url(url) {
            errors.push(("package.repository", format!("invalid HTTPS URL: {url}")));
        }
    }
}

fn validate_trust_metadata(trust: &ManifestTrust, errors: &mut Vec<(&'static str, String)>) {
    if let Some(level) = &trust.level {
        if TrustLevel::parse(level).is_none() {
            errors.push((
                "package.trust.level",
                format!("unknown trust level: {level}"),
            ));
        }
    }
    // Signatures and public keys must be base64 if present.
    if let Some(sig) = &trust.signature {
        if BASE64.decode(sig).is_err() {
            errors.push((
                "package.trust.signature",
                "signature must be valid base64".into(),
            ));
        }
    }
    if let Some(pk) = &trust.public_key {
        if BASE64.decode(pk).is_err() {
            errors.push((
                "package.trust.public_key",
                "public key must be valid base64".into(),
            ));
        }
    }
    if let Some(hash) = &trust.content_hash {
        if !hash.starts_with("sha256:") {
            errors.push((
                "package.trust.content_hash",
                "content_hash must use sha256: prefix".into(),
            ));
        }
    }
}

fn validate_package_trust(trust: &TrustInfo, errors: &mut Vec<(&'static str, String)>) {
    match (&trust.signature, &trust.public_key) {
        (Some(_), None) | (None, Some(_)) => errors.push((
            "trust.signature",
            "signature and public_key must be provided together".into(),
        )),
        (Some(signature), Some(public_key)) => {
            if BASE64.decode(signature).is_err() {
                errors.push(("trust.signature", "signature must be valid base64".into()));
            }
            if BASE64.decode(public_key).is_err() {
                errors.push(("trust.public_key", "public_key must be valid base64".into()));
            }
        }
        (None, None) => {}
    }

    if let Some(content_hash) = &trust.content_hash {
        let digest = content_hash.strip_prefix("sha256:");
        if !matches!(digest, Some(value) if value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        {
            errors.push((
                "trust.content_hash",
                "content_hash must be a sha256: digest with 64 hexadecimal characters".into(),
            ));
        }
    }
}
fn validate_metadata(
    meta: &crate::models::ManifestMetadata,
    errors: &mut Vec<(&'static str, String)>,
) {
    if meta.tags.len() > 50 {
        errors.push(("metadata.tags", "too many tags (max 50)".into()));
    }
    for (i, t) in meta.tags.iter().enumerate() {
        if t.len() > 32 {
            errors.push(("metadata.tags", format!("tag at index {i} too long")));
        }
    }
}

fn reject_unknown_security_fields(
    manifest: &PalaceManifest,
    errors: &mut Vec<(&'static str, String)>,
) {
    // Known top-level keys. Reject anything else at the top level as a security-critical guard.
    let known: HashSet<&str> = [
        "package",
        "capabilities",
        "metadata",
        "compatibility",
        "dependencies",
    ]
    .iter()
    .cloned()
    .collect();
    for key in manifest.extra.keys() {
        if !known.contains(key.as_str()) {
            errors.push(("unknown", format!("unknown top-level field: {key}")));
        }
    }
}

pub fn is_valid_https_url(url: &str) -> bool {
    if let Ok(parsed) = Url::parse(url) {
        parsed.scheme() == "https" && parsed.has_host()
    } else {
        false
    }
}

pub fn normalize_trust_level(client: Option<&str>) -> TrustLevel {
    // Clients cannot self-assign server-assigned levels.
    match client.and_then(TrustLevel::parse) {
        Some(level) if !level.is_server_assigned() => level,
        _ => TrustLevel::Experimental,
    }
}
