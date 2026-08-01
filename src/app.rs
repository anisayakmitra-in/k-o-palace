//! Application state and startup.

use crate::{
    artifact::ArtifactStorage,
    config::PalaceConfig,
    models::Package,
    rate_limit::RateLimiters,
    repository::{memory::InMemoryRepository, PackageRepository},
};
use std::sync::Arc;

/// Application state shared by all request handlers.
#[derive(Clone)]
pub struct AppState {
    pub config: PalaceConfig,
    pub repo: PackageRepository,
    pub storage: ArtifactStorage,
    pub rate_limiters: Arc<RateLimiters>,
}

impl AppState {
    /// Create a new in-memory app for tests and development.
    pub fn in_memory(config: PalaceConfig) -> Self {
        let repo = PackageRepository::Memory(InMemoryRepository::new());
        let storage = ArtifactStorage::from_config(&config);
        let rate_limiters = Arc::new(RateLimiters::from_config(&config));
        Self {
            config,
            repo,
            storage,
            rate_limiters,
        }
    }

    /// Seed the repository with sample packages.
    pub async fn seed_samples(&self) -> Result<(), crate::error::PalaceError> {
        for pkg in sample_packages() {
            self.repo.publish_package(&pkg).await?;
        }
        Ok(())
    }
}

fn sample_packages() -> Vec<Package> {
    use crate::models::{CapabilityInfo, CompatibilityInfo, PackageKind, TrustInfo, TrustLevel};
    let now = chrono::Utc::now();
    vec![
        Package {
            id: "browser.chrome".into(),
            name: "Chrome Browser Gene".into(),
            version: "1.4.0".into(),
            kind: PackageKind::Gene,
            description: "Browser automation gene using Chrome DevTools Protocol".into(),
            author: "openpandora".into(),
            license: "MIT".into(),
            trust: TrustInfo {
                level: TrustLevel::Official,
                signature: None,
                public_key: None,
                content_hash: None,
                publisher: "openpandora".into(),
            },
            capabilities: CapabilityInfo {
                provides: vec![
                    "browser.open".into(),
                    "browser.click".into(),
                    "browser.extract".into(),
                ],
                requires: vec![],
            },
            downloads: 15420,
            success_rate: 0.97,
            compatibility: CompatibilityInfo {
                runtimes: vec!["pandora>=0.2".into()],
                platforms: vec!["linux".into(), "macos".into(), "windows".into()],
            },
            artifact_url: None,
            repository: Some("https://github.com/openpandora/browser-gene".into()),
            homepage: None,
            tags: vec!["browser".into(), "automation".into(), "multimodal".into()],
            yanked: false,
            provenance: None,
            deprecated: None,
            created_at: now,
            updated_at: now,
        },
        Package {
            id: "filesystem.gene".into(),
            name: "Filesystem Gene".into(),
            version: "1.0.0".into(),
            kind: PackageKind::Gene,
            description: "File system operations — read, write, list, search".into(),
            author: "openpandora".into(),
            license: "MIT".into(),
            trust: TrustInfo {
                level: TrustLevel::Official,
                signature: None,
                public_key: None,
                content_hash: None,
                publisher: "openpandora".into(),
            },
            capabilities: CapabilityInfo {
                provides: vec!["filesystem.read".into(), "filesystem.write".into()],
                requires: vec![],
            },
            downloads: 28930,
            success_rate: 0.99,
            compatibility: CompatibilityInfo {
                runtimes: vec!["pandora>=0.2".into()],
                platforms: vec!["linux".into(), "macos".into(), "windows".into()],
            },
            artifact_url: None,
            repository: Some("https://github.com/openpandora/filesystem-gene".into()),
            homepage: None,
            tags: vec!["filesystem".into(), "tool".into(), "infrastructure".into()],
            yanked: false,
            provenance: None,
            deprecated: None,
            created_at: now,
            updated_at: now,
        },
    ]
}

#[cfg(feature = "postgres")]
impl AppState {
    pub async fn postgres(config: PalaceConfig) -> Result<Self, crate::error::PalaceError> {
        if config.database.url.trim().is_empty() {
            return Err(crate::error::PalaceError::new(
                crate::error::PalaceErrorCode::ServerError,
                "DATABASE_URL is required for the PostgreSQL backend",
            ));
        }
        let storage = ArtifactStorage::from_config(&config);
        storage.ensure_supported()?;
        let repository = crate::repository::postgres::PostgresRepository::new_with_max_connections(
            &config.database.url,
            config.database.max_connections,
        )
        .await?;
        repository.migrate().await?;
        Ok(Self {
            storage,
            rate_limiters: Arc::new(RateLimiters::from_config(&config)),
            config,
            repo: PackageRepository::Postgres(repository),
        })
    }
}
