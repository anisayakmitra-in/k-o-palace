//! Validation tests.

use k_o_palace::{
    error::PalaceErrorCode,
    models::{
        CapabilityInfo, CompatibilityInfo, ManifestPackage, ManifestTrust, Package, PackageKind,
        PalaceManifest, TrustInfo,
    },
    validation::{is_valid_https_url, validate_manifest, validate_package},
};

fn valid_manifest() -> PalaceManifest {
    PalaceManifest {
        package: ManifestPackage {
            id: "test.gene".into(),
            name: "Test Gene".into(),
            version: "1.0.0".into(),
            author: "tester".into(),
            description: "A test gene".into(),
            license: "MIT".into(),
            homepage: Some("https://example.com".into()),
            repository: Some("https://github.com/test/test".into()),
            kind: "gene".into(),
            trust: ManifestTrust::default(),
        },
        capabilities: CapabilityInfo::default(),
        metadata: Default::default(),
        compatibility: CompatibilityInfo::default(),
        extra: Default::default(),
    }
}

#[test]
fn valid_manifest_passes() {
    let result = validate_manifest(&valid_manifest());
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn invalid_id_fails() {
    let mut m = valid_manifest();
    m.package.id = "bad id".into();
    let err = validate_manifest(&m).unwrap_err();
    assert_eq!(err.code, PalaceErrorCode::InvalidManifest);
}

#[test]
fn invalid_version_fails() {
    let mut m = valid_manifest();
    m.package.version = "not-semver".into();
    let err = validate_manifest(&m).unwrap_err();
    assert_eq!(err.code, PalaceErrorCode::InvalidManifest);
}

#[test]
fn invalid_kind_fails() {
    let mut m = valid_manifest();
    m.package.kind = "unknown_kind".into();
    let err = validate_manifest(&m).unwrap_err();
    assert_eq!(err.code, PalaceErrorCode::InvalidManifest);
}

#[test]
fn http_url_rejected() {
    assert!(!is_valid_https_url("http://example.com"));
    assert!(!is_valid_https_url("ftp://example.com"));
    assert!(is_valid_https_url("https://example.com"));
}

#[test]
fn empty_name_fails() {
    let mut pkg = valid_package();
    pkg.name = "   ".into();
    let err = validate_package(&pkg).unwrap_err();
    assert_eq!(err.code, PalaceErrorCode::ValidationFailed);
}

fn valid_package() -> Package {
    use chrono::Utc;
    Package {
        id: "test.gene".into(),
        name: "Test Gene".into(),
        version: "1.0.0".into(),
        kind: PackageKind::Gene,
        description: "A test gene".into(),
        author: "tester".into(),
        license: "MIT".into(),
        trust: TrustInfo {
            level: k_o_palace::models::TrustLevel::Community,
            signature: None,
            public_key: None,
            content_hash: None,
            publisher: "tester".into(),
        },
        capabilities: CapabilityInfo::default(),
        downloads: 0,
        success_rate: 0.0,
        compatibility: CompatibilityInfo::default(),
        repository: Some("https://github.com/test/test".into()),
        artifact_url: None,
        homepage: None,
        tags: vec![],
        yanked: false,
        provenance: None,
        deprecated: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}
