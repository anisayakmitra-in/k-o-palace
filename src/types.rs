//! K-O Palace types — package metadata, trust, compatibility.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A KUBER package — any publishable AI runtime component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub id: String,
    pub name: String,
    pub version: String,
    pub kind: PackageKind,
    pub description: String,
    pub author: String,
    pub license: String,
    pub trust: TrustInfo,
    pub capabilities: CapabilityInfo,
    pub downloads: u64,
    pub success_rate: f64,
    pub compatibility: CompatibilityInfo,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Trust metadata for a package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustInfo {
    pub level: TrustLevel,
    pub signature: Option<String>,
    pub content_hash: Option<String>,
    pub publisher: String,
}

/// Trust levels — from experimental to certified.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrustLevel {
    Experimental,
    Community,
    Verified,
    Official,
    Enterprise,
    Certified,
}

impl TrustLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Experimental => "experimental",
            Self::Community => "community",
            Self::Verified => "verified",
            Self::Official => "official",
            Self::Enterprise => "enterprise",
            Self::Certified => "certified",
        }
    }
}

/// What a package provides and requires (capability-based dependencies).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CapabilityInfo {
    pub provides: Vec<String>,
    pub requires: Vec<String>,
}

/// Runtime and platform compatibility.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompatibilityInfo {
    pub runtimes: Vec<String>,
    pub platforms: Vec<String>,
}

/// What kind of artifact a package is.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PackageKind {
    Gene,
    DomainHarness,
    MetaHarness,
    SourceHarness,
    Package,
    Provider,
    Skill,
    MemorySchema,
    RuntimeExtension,
    CapabilityPack,
    Template,
    Persona,
    Policy,
    Benchmark,
    Dataset,
    Plugin,
    Connector,
    Sdk,
    Distribution,
}

impl PackageKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Gene => "gene",
            Self::DomainHarness => "domain_harness",
            Self::MetaHarness => "meta_harness",
            Self::SourceHarness => "source_harness",
            Self::Package => "package",
            Self::Provider => "provider",
            Self::Skill => "skill",
            Self::MemorySchema => "memory_schema",
            Self::RuntimeExtension => "runtime_extension",
            Self::CapabilityPack => "capability_pack",
            Self::Template => "template",
            Self::Persona => "persona",
            Self::Policy => "policy",
            Self::Benchmark => "benchmark",
            Self::Dataset => "dataset",
            Self::Plugin => "plugin",
            Self::Connector => "connector",
            Self::Sdk => "sdk",
            Self::Distribution => "distribution",
        }
    }
}

// ── API Types ──

#[derive(Debug, Deserialize)]
pub struct ListParams {
    pub q: Option<String>,
    pub kind: Option<String>,
    pub category: Option<String>,
    pub runtime: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
}

#[derive(Debug, Serialize)]
pub struct PackageListResponse {
    pub total: usize,
    pub packages: Vec<Package>,
}
