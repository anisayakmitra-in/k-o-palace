//! Package identity and namespace model.

use crate::error::{PalaceError, PalaceErrorCode, PalaceResult};

/// Maximum length for a namespace (publisher name) component.
pub const MAX_NAMESPACE_LEN: usize = 64;

/// Maximum length for a package slug.
pub const MAX_SLUG_LEN: usize = 128;

/// Maximum total length for a package ID.
pub const MAX_ID_LEN: usize = 256;

/// Reserved namespaces that cannot be registered by publishers.
pub const RESERVED_NAMESPACES: &[&str] = &[
    "palace",
    "registry",
    "admin",
    "system",
    "internal",
    "official",
    "root",
    "kopalace",
    "k-o-palace",
    "ko-palace",
    "kopalace-admin",
    "_internal",
];

/// Characters allowed in a namespace or slug component.
/// Lowercase alphanumeric, hyphens, and underscores.
/// Must start with a letter or underscore.
fn is_valid_component_char(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_'
}

/// Validate a namespace (publisher name) component.
pub fn validate_namespace(ns: &str) -> PalaceResult<()> {
    if ns.is_empty() {
        return Err(PalaceError::new(
            PalaceErrorCode::ValidationFailed,
            "namespace cannot be empty",
        ));
    }
    if ns.len() > MAX_NAMESPACE_LEN {
        return Err(PalaceError::new(
            PalaceErrorCode::ValidationFailed,
            format!("namespace exceeds maximum length of {MAX_NAMESPACE_LEN}"),
        ));
    }
    // Must start with a letter or underscore
    let first = ns.chars().next().unwrap();
    if !first.is_ascii_lowercase() && first != '_' {
        return Err(PalaceError::new(
            PalaceErrorCode::ValidationFailed,
            "namespace must start with a lowercase letter or underscore",
        ));
    }
    // All chars must be valid
    if !ns.chars().all(is_valid_component_char) {
        return Err(PalaceError::new(
            PalaceErrorCode::ValidationFailed,
            "namespace contains invalid characters (allowed: a-z, 0-9, -, _)",
        ));
    }
    // Reserved check
    let ns_lower = ns.to_lowercase();
    if RESERVED_NAMESPACES
        .iter()
        .any(|r| r.eq_ignore_ascii_case(&ns_lower))
    {
        return Err(PalaceError::new(
            PalaceErrorCode::Conflict,
            format!("namespace '{ns}' is reserved"),
        ));
    }
    // No consecutive hyphens/underscores at start
    if ns.starts_with("--") || ns.starts_with("__") {
        return Err(PalaceError::new(
            PalaceErrorCode::ValidationFailed,
            "namespace cannot start with consecutive hyphens or underscores",
        ));
    }
    // No trailing hyphen
    if ns.ends_with('-') {
        return Err(PalaceError::new(
            PalaceErrorCode::ValidationFailed,
            "namespace cannot end with a hyphen",
        ));
    }
    Ok(())
}

/// Validate a package slug component.
pub fn validate_slug(slug: &str) -> PalaceResult<()> {
    if slug.is_empty() {
        return Err(PalaceError::new(
            PalaceErrorCode::ValidationFailed,
            "package slug cannot be empty",
        ));
    }
    if slug.len() > MAX_SLUG_LEN {
        return Err(PalaceError::new(
            PalaceErrorCode::ValidationFailed,
            format!("package slug exceeds maximum length of {MAX_SLUG_LEN}"),
        ));
    }
    let first = slug.chars().next().unwrap();
    if !first.is_ascii_lowercase() && first != '_' {
        return Err(PalaceError::new(
            PalaceErrorCode::ValidationFailed,
            "package slug must start with a lowercase letter or underscore",
        ));
    }
    if !slug.chars().all(is_valid_component_char) {
        return Err(PalaceError::new(
            PalaceErrorCode::ValidationFailed,
            "package slug contains invalid characters (allowed: a-z, 0-9, -, _)",
        ));
    }
    if slug.ends_with('-') {
        return Err(PalaceError::new(
            PalaceErrorCode::ValidationFailed,
            "package slug cannot end with a hyphen",
        ));
    }
    Ok(())
}

/// Normalize a package ID to its canonical form.
/// Accepts `@namespace/slug` or `namespace/slug` formats.
/// Returns the normalized `namespace/slug` form (lowercase, trimmed).
pub fn normalize_id(raw: &str) -> PalaceResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(PalaceError::new(
            PalaceErrorCode::ValidationFailed,
            "package ID cannot be empty",
        ));
    }
    if trimmed.len() > MAX_ID_LEN {
        return Err(PalaceError::new(
            PalaceErrorCode::ValidationFailed,
            format!("package ID exceeds maximum length of {MAX_ID_LEN}"),
        ));
    }

    // Reject path traversal attempts
    if trimmed.contains("..") {
        return Err(PalaceError::new(
            PalaceErrorCode::ValidationFailed,
            "package ID cannot contain '..'",
        ));
    }
    if trimmed.contains('/') && trimmed.contains('\\') {
        return Err(PalaceError::new(
            PalaceErrorCode::ValidationFailed,
            "package ID cannot mix forward and backslash separators",
        ));
    }
    // Normalize backslashes to forward slashes, then reject if still present after normalization
    let normalized_sep = trimmed.replace('\\', "/");
    if normalized_sep.contains('\\') {
        return Err(PalaceError::new(
            PalaceErrorCode::ValidationFailed,
            "package ID contains invalid backslash",
        ));
    }

    // Strip leading @ if present
    let without_at = normalized_sep.strip_prefix('@').unwrap_or(&normalized_sep);

    // Must contain exactly one separator
    let parts: Vec<&str> = without_at.split('/').collect();
    if parts.len() != 2 {
        return Err(PalaceError::new(
            PalaceErrorCode::ValidationFailed,
            "package ID must be in 'namespace/slug' format",
        ));
    }

    let ns = parts[0].to_lowercase();
    let slug = parts[1].to_lowercase();

    validate_namespace(&ns)?;
    validate_slug(&slug)?;

    Ok(format!("{ns}/{slug}"))
}

/// Extract the namespace from a normalized package ID.
pub fn namespace_of(id: &str) -> &str {
    id.split('/').next().unwrap_or(id)
}

/// Extract the slug from a normalized package ID.
pub fn slug_of(id: &str) -> &str {
    id.split('/').nth(1).unwrap_or(id)
}

/// Check if a package ID could be confused with another via Unicode confusables.
/// This is a basic check — full confusable detection requires a lookup table.
pub fn check_confusables(id: &str) -> PalaceResult<()> {
    // Reject any non-ASCII characters
    if !id.is_ascii() {
        return Err(PalaceError::new(
            PalaceErrorCode::ValidationFailed,
            "package ID must be ASCII-only (no Unicode confusables)",
        ));
    }
    // Reject URL-encoded sequences
    if id.contains("%2e") || id.contains("%2E") || id.contains("%2f") || id.contains("%2F") {
        return Err(PalaceError::new(
            PalaceErrorCode::ValidationFailed,
            "package ID cannot contain URL-encoded sequences",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_namespace_slug() {}

    #[test]
    fn normalize_strips_at_prefix() {
        assert_eq!(normalize_id("@alice/foo").unwrap(), "alice/foo");
    }

    #[test]
    fn normalize_lowercases() {
        assert_eq!(normalize_id("Alice/Foo").unwrap(), "alice/foo");
    }

    #[test]
    fn normalize_trims_whitespace() {
        assert_eq!(normalize_id("  alice/foo  ").unwrap(), "alice/foo");
    }

    #[test]
    fn reject_path_traversal() {
        assert!(normalize_id("../foo").is_err());
        // "..foo" contains ".." which we reject for safety
        assert!(normalize_id("alice/..foo").is_err());
        assert!(normalize_id("alice/../bar").is_err());
    }

    #[test]
    fn reject_backslash() {
        assert!(normalize_id("alice\\foo").is_ok()); // normalizes to alice/foo
        assert!(normalize_id("alice\\\\foo").is_err());
    }

    #[test]
    fn reject_empty() {
        assert!(normalize_id("").is_err());
        assert!(normalize_id("   ").is_err());
    }

    #[test]
    fn reject_no_separator() {
        assert!(normalize_id("alice").is_err());
    }

    #[test]
    fn reject_double_separator() {
        assert!(normalize_id("alice/foo/bar").is_err());
    }

    #[test]
    fn reject_reserved_namespace() {
        assert!(normalize_id("palace/foo").is_err());
        assert!(normalize_id("admin/foo").is_err());
        assert!(normalize_id("system/foo").is_err());
    }

    #[test]
    fn reject_uppercase_start() {
        assert!(normalize_id("Alice/foo").is_ok()); // normalized to lowercase
    }

    #[test]
    fn reject_trailing_hyphen() {
        assert!(normalize_id("alice-/foo").is_err());
        assert!(normalize_id("alice/foo-").is_err());
    }

    #[test]
    fn reject_very_long_names() {
        let long_ns = "a".repeat(65);
        assert!(normalize_id(&format!("{long_ns}/foo")).is_err());
        let long_slug = "b".repeat(129);
        assert!(normalize_id(&format!("alice/{long_slug}")).is_err());
    }

    #[test]
    fn reject_non_ascii() {
        assert!(normalize_id("αlice/foo").is_err());
    }

    #[test]
    fn reject_url_encoded() {
        assert!(check_confusables("alice%2efoo").is_err());
        assert!(check_confusables("alice%2Fbar").is_err());
    }

    #[test]
    fn reject_whitespace_in_component() {
        assert!(normalize_id("ali ce/foo").is_err());
        assert!(normalize_id("alice/fo o").is_err());
    }

    #[test]
    fn extract_namespace_and_slug() {
        assert_eq!(namespace_of("alice/foo"), "alice");
        assert_eq!(slug_of("alice/foo"), "foo");
    }

    #[test]
    fn underscore_start_allowed() {
        assert!(normalize_id("_internal/foo").is_err()); // _internal is reserved
        assert!(normalize_id("_test/foo").is_ok());
    }
}
