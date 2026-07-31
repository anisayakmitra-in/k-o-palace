//! Artifact host validation and security tests.

use k_o_palace::{artifact::validate_artifact_url, config::PalaceConfig};

fn config_with_hosts(hosts: Vec<String>) -> PalaceConfig {
    let mut cfg = PalaceConfig::default();
    cfg.storage.allowed_hosts = hosts;
    cfg
}

#[test]
fn https_url_on_allowlisted_host_passes() {
    let cfg = config_with_hosts(vec!["github.com".into()]);
    assert!(validate_artifact_url(
        "https://github.com/test/test/releases/download/v1.0.0/pkg.tar.gz",
        &cfg
    )
    .is_ok());
}

#[test]
fn http_url_rejected_in_production() {
    let cfg = config_with_hosts(vec!["github.com".into()]);
    let err = validate_artifact_url("http://github.com/test/test/pkg.tar.gz", &cfg).unwrap_err();
    assert_eq!(err.code, k_o_palace::error::PalaceErrorCode::InsecureUrl);
}

#[test]
fn url_not_on_allowlist_rejected() {
    let cfg = config_with_hosts(vec!["github.com".into()]);
    let err = validate_artifact_url("https://evil.com/pkg.tar.gz", &cfg).unwrap_err();
    assert_eq!(
        err.code,
        k_o_palace::error::PalaceErrorCode::ArtifactNotAllowed
    );
}

#[test]
fn default_allowlist_used_when_empty() {
    let cfg = config_with_hosts(vec![]);
    assert!(validate_artifact_url("https://github.com/test/test/pkg.tar.gz", &cfg).is_ok());
    assert!(validate_artifact_url(
        "https://objects.githubusercontent.com/test/pkg.tar.gz",
        &cfg
    )
    .is_ok());
    assert!(validate_artifact_url("https://evil.com/pkg.tar.gz", &cfg).is_err());
}

#[test]
fn invalid_url_rejected() {
    let cfg = config_with_hosts(vec![]);
    assert!(validate_artifact_url("not-a-url", &cfg).is_err());
}

#[test]
fn content_hash_computes_correctly() {
    use k_o_palace::artifact::compute_content_hash;
    let hash = compute_content_hash(b"hello world");
    assert_eq!(
        hash,
        "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    );
}
