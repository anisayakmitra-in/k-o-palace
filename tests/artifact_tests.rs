//! Artifact host validation and security tests.

use k_o_palace::{
    artifact::{validate_artifact_url, verify_artifact_content},
    config::PalaceConfig,
    models::{TrustInfo, TrustLevel},
};

#[cfg(feature = "reqwest")]
use k_o_palace::{artifact::fetch_and_verify_with_config, error::PalaceErrorCode};

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
fn allowlisted_loopback_address_is_rejected() {
    let cfg = config_with_hosts(vec!["127.0.0.1".into()]);
    let err = validate_artifact_url("https://127.0.0.1/pkg.tar.gz", &cfg).unwrap_err();
    assert_eq!(
        err.code,
        k_o_palace::error::PalaceErrorCode::ArtifactNotAllowed
    );
}

#[cfg(feature = "reqwest")]
#[tokio::test]
async fn fetch_rejects_allowlisted_localhost_before_connecting() {
    let cfg = config_with_hosts(vec!["localhost".into()]);
    let err = fetch_and_verify_with_config("https://localhost/pkg.tar.gz", None, &cfg)
        .await
        .unwrap_err();
    assert_eq!(err.code, PalaceErrorCode::ArtifactNotAllowed);
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

#[test]
fn verified_artifact_content_requires_the_declared_digest() {
    let trust = TrustInfo {
        level: TrustLevel::Community,
        signature: None,
        public_key: None,
        content_hash: Some(
            "sha256:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9".into(),
        ),
        publisher: "test".into(),
    };

    let info =
        verify_artifact_content(b"hello world", Some("application/gzip".into()), &trust).unwrap();
    assert_eq!(info.size, 11);
    assert_eq!(info.content_type.as_deref(), Some("application/gzip"));
}

#[test]
fn verified_artifact_content_rejects_a_digest_mismatch() {
    let trust = TrustInfo {
        level: TrustLevel::Community,
        signature: None,
        public_key: None,
        content_hash: Some(
            "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
        ),
        publisher: "test".into(),
    };

    let error = verify_artifact_content(b"hello world", None, &trust).unwrap_err();
    assert_eq!(error.code, k_o_palace::error::PalaceErrorCode::HashMismatch);
}
