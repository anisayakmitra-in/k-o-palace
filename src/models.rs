//! Domain models for K-O Palace.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

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
    #[serde(default)]
    pub artifact_url: Option<String>,
    pub homepage: Option<String>,
    pub tags: Vec<String>,
    /// Whether this package version has been yanked (hidden from install but not deleted).
    #[serde(default)]
    pub yanked: bool,
    /// Optional deprecation message (set when a package is deprecated).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<String>,
    /// Git forge provenance metadata (commit SHA, tag, release ID).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Trust metadata for a package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustInfo {
    pub level: TrustLevel,
    pub signature: Option<String>,
    #[serde(default)]
    pub public_key: Option<String>,
    pub content_hash: Option<String>,
    pub publisher: String,
}

/// Trust levels — from experimental to certified.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum TrustLevel {
    #[serde(alias = "experimental")]
    Experimental,
    #[serde(alias = "community")]
    Community,
    #[serde(alias = "verified")]
    Verified,
    #[serde(alias = "official")]
    Official,
    #[serde(alias = "enterprise")]
    Enterprise,
    #[default]
    #[serde(alias = "certified")]
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

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "experimental" => Some(Self::Experimental),
            "community" => Some(Self::Community),
            "verified" => Some(Self::Verified),
            "official" => Some(Self::Official),
            "enterprise" => Some(Self::Enterprise),
            "certified" => Some(Self::Certified),
            _ => None,
        }
    }

    /// Server-assigned levels that clients cannot self-assign.
    pub fn is_server_assigned(&self) -> bool {
        matches!(
            self,
            Self::Verified | Self::Official | Self::Enterprise | Self::Certified
        )
    }

    /// Numeric rank for transition ordering.
    pub fn rank(&self) -> u8 {
        match self {
            Self::Experimental => 0,
            Self::Community => 1,
            Self::Verified => 2,
            Self::Official => 3,
            Self::Enterprise => 4,
            Self::Certified => 5,
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
#[serde(rename_all = "snake_case")]
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

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "gene" => Some(Self::Gene),
            "domain_harness" => Some(Self::DomainHarness),
            "meta_harness" => Some(Self::MetaHarness),
            "source_harness" => Some(Self::SourceHarness),
            "package" => Some(Self::Package),
            "provider" => Some(Self::Provider),
            "skill" => Some(Self::Skill),
            "memory_schema" => Some(Self::MemorySchema),
            "runtime_extension" => Some(Self::RuntimeExtension),
            "capability_pack" => Some(Self::CapabilityPack),
            "template" => Some(Self::Template),
            "persona" => Some(Self::Persona),
            "policy" => Some(Self::Policy),
            "benchmark" => Some(Self::Benchmark),
            "dataset" => Some(Self::Dataset),
            "plugin" => Some(Self::Plugin),
            "connector" => Some(Self::Connector),
            "sdk" => Some(Self::Sdk),
            "distribution" => Some(Self::Distribution),
            _ => None,
        }
    }
}

// ── API Types ──

#[derive(Debug, Deserialize, Default)]
pub struct ListParams {
    pub q: Option<String>,
    pub kind: Option<String>,
    pub category: Option<String>,
    pub runtime: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    50
}

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageListResponse {
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
    pub packages: Vec<Package>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionListResponse {
    pub total: usize,
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Review {
    pub id: Uuid,
    pub package_id: String,
    pub reviewer_id: Uuid,
    pub rating: i16,
    pub comment: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ReviewRequest {
    pub rating: i16,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Publisher {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub email: Option<String>,
    pub website: Option<String>,
    pub role: Role,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Publisher,
    Maintainer,
    Moderator,
    Administrator,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Publisher => "publisher",
            Self::Maintainer => "maintainer",
            Self::Moderator => "moderator",
            Self::Administrator => "administrator",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "publisher" => Some(Self::Publisher),
            "maintainer" => Some(Self::Maintainer),
            "moderator" => Some(Self::Moderator),
            "administrator" => Some(Self::Administrator),
            _ => None,
        }
    }

    /// Whether this role can publish packages under a publisher account.
    pub fn can_publish(&self) -> bool {
        matches!(
            self,
            Self::Publisher | Self::Maintainer | Self::Moderator | Self::Administrator
        )
    }

    /// Whether this role can moderate reviews and trust transitions.
    pub fn can_moderate(&self) -> bool {
        matches!(self, Self::Moderator | Self::Administrator)
    }

    /// Whether this role can administer publishers and tokens.
    pub fn can_administer(&self) -> bool {
        matches!(self, Self::Administrator)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiToken {
    pub id: Uuid,
    pub publisher_id: Uuid,
    pub name: String,
    pub token_hash: String,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustTransition {
    pub id: Uuid,
    pub package_id: String,
    pub from_level: String,
    pub to_level: String,
    pub approved_by: Uuid,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct TrustTransitionRequest {
    pub level: TrustLevel,
    pub reason: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub event_type: String,
    pub actor_id: Option<Uuid>,
    pub package_id: Option<String>,
    pub details: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Parsed and normalized manifest for a package version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub package_id: String,
    pub version: String,
    pub raw: String,
    pub parsed: KuberManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KuberManifest {
    pub package: ManifestPackage,
    #[serde(default)]
    pub capabilities: CapabilityInfo,
    #[serde(default)]
    pub metadata: ManifestMetadata,
    #[serde(default)]
    pub compatibility: CompatibilityInfo,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestPackage {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub license: String,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub kind: String,
    #[serde(default)]
    pub trust: ManifestTrust,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ManifestTrust {
    pub level: Option<String>,
    pub signature: Option<String>,
    pub public_key: Option<String>,
    pub content_hash: Option<String>,
    pub publisher: Option<String>,
    pub min_runtime_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ManifestMetadata {
    #[serde(default)]
    pub tags: Vec<String>,
    pub icon: Option<String>,
    pub documentation: Option<String>,
    #[serde(default)]
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub artifact_url: Option<String>,
    pub content_hash: Option<String>,
}

// ── Publisher Management API Types ──

/// Request body for publisher registration.
#[derive(Debug, Deserialize)]
pub struct PublisherRegisterRequest {
    pub name: String,
    pub display_name: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub website: Option<String>,
}

/// Public publisher response (no sensitive data).
#[derive(Debug, Serialize)]
pub struct PublisherResponse {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub website: Option<String>,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

impl From<Publisher> for PublisherResponse {
    fn from(p: Publisher) -> Self {
        Self {
            id: p.id,
            name: p.name,
            display_name: p.display_name,
            website: p.website,
            role: p.role.as_str().to_string(),
            created_at: p.created_at,
        }
    }
}

/// Response from publisher registration (includes token shown once).
#[derive(Debug, Serialize)]
pub struct PublisherRegisterResponse {
    pub publisher: PublisherResponse,
    pub token: String,
}

/// Request body for token creation.
#[derive(Debug, Deserialize)]
pub struct TokenCreateRequest {
    pub name: String,
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub scopes: Vec<String>,
}

/// Public token response (never includes token_hash).
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub scopes: Vec<String>,
}

impl From<ApiToken> for TokenResponse {
    fn from(t: ApiToken) -> Self {
        Self {
            id: t.id,
            name: t.name,
            created_at: t.created_at,
            revoked_at: t.revoked_at,
            expires_at: t.expires_at,
            scopes: t.scopes,
        }
    }
}

/// Response from token creation (includes plaintext token shown once).
#[derive(Debug, Serialize)]
pub struct TokenCreateResponse {
    pub token: String,
    pub token_info: TokenResponse,
}

/// Git forge provenance for a package version.
/// Captures immutable source identity — never use branch names as provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    /// Forge type: "github", "gitlab", "codeberg", "forgejo", "gitea", "git"
    pub forge: String,
    /// Full repository URL (HTTPS)
    pub repository_url: String,
    /// Repository owner/org
    pub owner: String,
    /// Repository name
    pub repo: String,
    /// Immutable commit SHA (never a branch name)
    pub commit_sha: String,
    /// Git tag if published from a tag
    pub tag: Option<String>,
    /// GitHub release ID if applicable
    pub release_id: Option<String>,
    /// Path to manifest file in the repository
    pub manifest_path: Option<String>,
    /// Digest of the manifest content
    pub manifest_digest: Option<String>,
}
