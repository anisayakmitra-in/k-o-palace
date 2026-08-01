use chrono::Utc;

pub mod models {
    pub use k_o_palace::models::*;
}

#[path = "../src/resolve.rs"]
mod resolve;

use k_o_palace::models::{
    CapabilityInfo, CompatibilityInfo, Package, PackageKind, TrustInfo, TrustLevel,
};
use resolve::{resolve_dependencies, ResolveOptions};

fn package(
    id: &str,
    version: &str,
    trust: TrustLevel,
    downloads: u64,
    provides: &[&str],
    requires: &[&str],
) -> Package {
    Package {
        id: id.into(),
        name: id.into(),
        version: version.into(),
        kind: PackageKind::Package,
        description: format!("{id} package"),
        author: "tester".into(),
        license: "MIT".into(),
        trust: TrustInfo {
            level: trust,
            signature: None,
            public_key: None,
            content_hash: None,
            publisher: "tester".into(),
        },
        capabilities: CapabilityInfo {
            provides: provides.iter().map(|value| (*value).to_string()).collect(),
            requires: requires.iter().map(|value| (*value).to_string()).collect(),
        },
        downloads,
        success_rate: 1.0,
        compatibility: CompatibilityInfo::default(),
        repository: None,
        artifact_url: None,
        homepage: None,
        tags: vec![],
        yanked: false,
        deprecated: None,
        provenance: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[test]
fn runtime_and_platform_filters_select_only_compatible_candidates() {
    let mut root = package(
        "root.pkg",
        "1.0.0",
        TrustLevel::Certified,
        1,
        &[],
        &["cap.db"],
    );
    root.compatibility = CompatibilityInfo {
        runtimes: vec!["runtime-a".into()],
        platforms: vec!["linux".into()],
    };

    let mut compatible = package(
        "compatible.db",
        "1.0.0",
        TrustLevel::Community,
        5,
        &["cap.db"],
        &[],
    );
    compatible.compatibility = CompatibilityInfo {
        runtimes: vec!["runtime-a".into()],
        platforms: vec!["linux".into()],
    };

    let mut wrong_runtime = package(
        "wrong-runtime.db",
        "9.0.0",
        TrustLevel::Certified,
        999,
        &["cap.db"],
        &[],
    );
    wrong_runtime.compatibility.runtimes = vec!["runtime-b".into()];

    let mut wrong_platform = package(
        "wrong-platform.db",
        "9.0.0",
        TrustLevel::Certified,
        999,
        &["cap.db"],
        &[],
    );
    wrong_platform.compatibility.platforms = vec!["windows".into()];

    let response = resolve_dependencies(
        &root,
        &[compatible, wrong_runtime, wrong_platform],
        ResolveOptions {
            runtime: Some("runtime-a".into()),
            platform: Some("linux".into()),
        },
    );

    assert!(response.complete);
    assert_eq!(response.root_compatibility.runtimes, vec!["runtime-a"]);
    assert_eq!(response.resolved_dependencies.len(), 1);
    let dependency = &response.resolved_dependencies[0];
    assert_eq!(dependency.capability, "cap.db");
    assert_eq!(
        dependency.selected_package_id.as_deref(),
        Some("compatible.db")
    );
    assert_eq!(dependency.selected_version.as_deref(), Some("1.0.0"));
    assert_eq!(dependency.selected_trust.as_deref(), Some("community"));
    assert!(response.missing_capabilities.is_empty());
    assert!(response.incompatible_capabilities.is_empty());
}

#[test]
fn highest_trust_then_valid_semver_then_downloads_are_used() {
    let root = package(
        "root.pkg",
        "1.0.0",
        TrustLevel::Certified,
        1,
        &[],
        &["cap.ai"],
    );

    let lower_trust_higher_version = package(
        "community.ai",
        "9.9.9",
        TrustLevel::Community,
        1_000,
        &["cap.ai"],
        &[],
    );

    let higher_trust_invalid_version = package(
        "official-invalid.ai",
        "not-semver",
        TrustLevel::Official,
        50_000,
        &["cap.ai"],
        &[],
    );

    let higher_trust_lower_downloads = package(
        "official.ai",
        "2.0.0",
        TrustLevel::Official,
        10,
        &["cap.ai"],
        &[],
    );

    let higher_trust_lower_version_more_downloads = package(
        "official-older.ai",
        "1.5.0",
        TrustLevel::Official,
        100_000,
        &["cap.ai"],
        &[],
    );

    let response = resolve_dependencies(
        &root,
        &[
            lower_trust_higher_version,
            higher_trust_invalid_version,
            higher_trust_lower_downloads,
            higher_trust_lower_version_more_downloads,
        ],
        ResolveOptions::default(),
    );

    let dependency = &response.resolved_dependencies[0];
    assert_eq!(
        dependency.selected_package_id.as_deref(),
        Some("official.ai")
    );
    assert_eq!(dependency.selected_version.as_deref(), Some("2.0.0"));
    assert_eq!(dependency.selected_trust.as_deref(), Some("official"));
    assert!(dependency.reason.contains("best"));
}

#[test]
fn missing_capabilities_are_reported_without_selection() {
    let root = package(
        "root.pkg",
        "1.0.0",
        TrustLevel::Certified,
        1,
        &[],
        &["cap.missing"],
    );

    let response = resolve_dependencies(&root, &[], ResolveOptions::default());

    assert!(!response.complete);
    assert_eq!(response.missing_capabilities, vec!["cap.missing"]);
    assert!(response.incompatible_capabilities.is_empty());
    assert_eq!(response.resolved_dependencies.len(), 1);
    let dependency = &response.resolved_dependencies[0];
    assert_eq!(dependency.capability, "cap.missing");
    assert!(dependency.selected_package_id.is_none());
    assert!(dependency.selected_version.is_none());
    assert!(dependency.selected_trust.is_none());
    assert!(dependency.reason.contains("no non-yanked package"));
}

#[test]
fn yanked_candidates_are_ignored() {
    let root = package(
        "root.pkg",
        "1.0.0",
        TrustLevel::Certified,
        1,
        &[],
        &["cap.fs"],
    );

    let mut yanked = package(
        "official.fs",
        "5.0.0",
        TrustLevel::Official,
        1_000_000,
        &["cap.fs"],
        &[],
    );
    yanked.yanked = true;

    let fallback = package(
        "community.fs",
        "1.0.0",
        TrustLevel::Community,
        10,
        &["cap.fs"],
        &[],
    );

    let response = resolve_dependencies(&root, &[yanked, fallback], ResolveOptions::default());

    let dependency = &response.resolved_dependencies[0];
    assert_eq!(
        dependency.selected_package_id.as_deref(),
        Some("community.fs")
    );
    assert_eq!(dependency.selected_trust.as_deref(), Some("community"));
}

#[test]
fn cycles_are_bounded_and_do_not_repeat_capabilities() {
    let root = package(
        "root.pkg",
        "1.0.0",
        TrustLevel::Certified,
        1,
        &[],
        &["cap.a"],
    );

    let a = package(
        "pkg.a",
        "1.0.0",
        TrustLevel::Official,
        10,
        &["cap.a"],
        &["cap.b"],
    );
    let b = package(
        "pkg.b",
        "1.0.0",
        TrustLevel::Official,
        10,
        &["cap.b"],
        &["cap.a"],
    );

    let response = resolve_dependencies(&root, &[a, b], ResolveOptions::default());

    assert!(response.complete);
    assert_eq!(response.resolved_dependencies.len(), 2);
    assert_eq!(response.resolved_dependencies[0].capability, "cap.a");
    assert_eq!(response.resolved_dependencies[1].capability, "cap.b");
    assert!(response.missing_capabilities.is_empty());
    assert!(response.incompatible_capabilities.is_empty());
}

#[test]
fn runtime_constraint_names_match_requested_runtime() {
    let root = package(
        "root.pkg",
        "1.0.0",
        TrustLevel::Certified,
        1,
        &[],
        &["cap.runtime"],
    );
    let mut runtime = package(
        "runtime.pkg",
        "1.0.0",
        TrustLevel::Official,
        10,
        &["cap.runtime"],
        &[],
    );
    runtime.compatibility.runtimes = vec!["pandora>=0.2".into()];

    let response = resolve_dependencies(
        &root,
        &[runtime],
        ResolveOptions {
            runtime: Some("pandora".into()),
            platform: None,
        },
    );

    assert!(response.complete);
    assert_eq!(
        response.resolved_dependencies[0]
            .selected_package_id
            .as_deref(),
        Some("runtime.pkg")
    );
}

#[test]
fn resolution_work_budget_does_not_expand_with_catalog_size() {
    let required: Vec<String> = (0..1025).map(|index| format!("cap.{index}")).collect();
    let mut root = package(
        "root.pkg",
        "1.0.0",
        TrustLevel::Certified,
        1,
        &[],
        &[],
    );
    root.capabilities.requires = required;

    let catalog: Vec<Package> = (0..300)
        .map(|index| {
            package(
                &format!("irrelevant.{index}"),
                "1.0.0",
                TrustLevel::Community,
                0,
                &[],
                &[],
            )
        })
        .collect();

    let response = resolve_dependencies(&root, &catalog, ResolveOptions::default());
    assert!(response.resolved_dependencies.iter().any(|dependency| {
        dependency.reason == "resolution work budget exceeded"
    }));
