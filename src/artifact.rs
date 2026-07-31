//! Artifact storage adapters.

use crate::config::{PalaceConfig, StorageBackend};
use crate::error::{PalaceError, PalaceErrorCode, PalaceResult};
use sha2::{Digest, Sha256};
use url::Url;

/// Metadata about a fetched artifact.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ArtifactInfo {
    pub content_type: Option<String>,
    pub size: usize,
    pub hash: String,
}

/// Storage backend trait.
pub trait ArtifactStorageBackend: Send + Sync {
    fn name(&self) -> &str;
}

/// Enum-dispatched storage backends.
#[derive(Debug, Clone)]
pub enum ArtifactStorage {
    Local(LocalFileStorage),
    GitHub(GitHubReleaseStorage),
    GitLab(GitHubReleaseStorage),
    Codeberg(GitHubReleaseStorage),
    Oci(GitHubReleaseStorage),
    S3(GitHubReleaseStorage),
    Azure(GitHubReleaseStorage),
    Gcs(GitHubReleaseStorage),
}

impl ArtifactStorage {
    /// Create a storage adapter from the configuration.
    pub fn from_config(config: &PalaceConfig) -> Self {
        match config.storage.backend {
            StorageBackend::Local => ArtifactStorage::Local(LocalFileStorage),
            StorageBackend::GitHub => ArtifactStorage::GitHub(GitHubReleaseStorage),
            StorageBackend::GitLab => ArtifactStorage::GitLab(GitHubReleaseStorage),
            StorageBackend::Codeberg => ArtifactStorage::Codeberg(GitHubReleaseStorage),
            StorageBackend::Oci => ArtifactStorage::Oci(GitHubReleaseStorage),
            StorageBackend::S3 => ArtifactStorage::S3(GitHubReleaseStorage),
            StorageBackend::Azure => ArtifactStorage::Azure(GitHubReleaseStorage),
            StorageBackend::Gcs => ArtifactStorage::Gcs(GitHubReleaseStorage),
        }
    }
}

/// Local filesystem storage backend (for tests and development).
#[derive(Debug, Clone, Default)]
pub struct LocalFileStorage;

/// GitHub Release metadata storage (validates HTTPS URLs, redirects, checksums).
#[derive(Debug, Clone, Default)]
pub struct GitHubReleaseStorage;

/// Validate an artifact URL against the allowlist and HTTPS requirement.
pub fn validate_artifact_url(url: &str, config: &PalaceConfig) -> PalaceResult<()> {
    let parsed = Url::parse(url)
        .map_err(|_| PalaceError::new(PalaceErrorCode::BadRequest, "invalid artifact URL"))?;

    if parsed.scheme() != "https" {
        return Err(PalaceError::new(
            PalaceErrorCode::InsecureUrl,
            "artifact URL must use HTTPS in production",
        ));
    }

    let host = parsed.host_str().unwrap_or("");
    let allowed: &[String] = if config.storage.allowed_hosts.is_empty() {
        &[
            "github.com".to_string(),
            "objects.githubusercontent.com".to_string(),
        ]
    } else {
        &config.storage.allowed_hosts
    };

    if !allowed.iter().any(|a| host == a.as_str()) {
        return Err(PalaceError::new(
            PalaceErrorCode::ArtifactNotAllowed,
            format!("host '{host}' is not in the allowed artifact host list"),
        ));
    }

    Ok(())
}

/// Compute the SHA-256 hash of artifact content.
pub fn compute_content_hash(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    hex::encode(hasher.finalize())
}

/// Fetch artifact content with redirect limit enforcement.
///
/// Requires the `reqwest` feature. Returns `NotImplemented` otherwise.
#[cfg(feature = "reqwest")]
pub async fn fetch_with_redirect_limit(url: &str, max_redirects: u32) -> PalaceResult<Vec<u8>> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(max_redirects as usize))
        .build()
        .map_err(|e| {
            PalaceError::new(
                PalaceErrorCode::StorageError,
                format!("failed to build HTTP client: {e}"),
            )
        })?;

    let resp = client.get(url).send().await.map_err(|e| {
        PalaceError::new(
            PalaceErrorCode::StorageError,
            format!("failed to fetch artifact: {e}"),
        )
    })?;

    if !resp.status().is_success() {
        return Err(PalaceError::new(
            PalaceErrorCode::StorageError,
            format!("artifact fetch failed: HTTP {}", resp.status()),
        ));
    }

    let content_type = resp.headers().get(reqwest::header::CONTENT_TYPE).cloned();
    let bytes = resp.bytes().await.map_err(|e| {
        PalaceError::new(
            PalaceErrorCode::StorageError,
            format!("failed to read artifact bytes: {e}"),
        )
    })?;

    if let Some(ct) = content_type {
        let ct = ct.to_str().unwrap_or("");
        if !ct.starts_with("application/")
            && !ct.starts_with("binary/")
            && !ct.starts_with("text/")
            && !ct.is_empty()
        {
            return Err(PalaceError::new(
                PalaceErrorCode::BadRequest,
                format!("unallowed content type for artifact: {ct}"),
            ));
        }
    }

    if bytes.len() > 100 * 1024 * 1024 {
        return Err(PalaceError::new(
            PalaceErrorCode::TooLarge,
            "artifact exceeds maximum size of 100 MB",
        ));
    }

    Ok(bytes.to_vec())
}

/// Fetch artifact content (stub when reqwest is not enabled).
#[cfg(not(feature = "reqwest"))]
pub async fn fetch_with_redirect_limit(_url: &str, _max_redirects: u32) -> PalaceResult<Vec<u8>> {
    Err(PalaceError::new(
        PalaceErrorCode::NotImplemented,
        "reqwest feature not enabled",
    ))
}

/// Fetch artifact metadata and verify its content hash.
pub async fn fetch_and_verify(
    url: &str,
    expected_hash: Option<&str>,
    max_redirects: u32,
) -> PalaceResult<ArtifactInfo> {
    let content = fetch_with_redirect_limit(url, max_redirects).await?;
    let hash = compute_content_hash(&content);

    if let Some(expected) = expected_hash {
        if hash != expected {
            return Err(PalaceError::new(
                PalaceErrorCode::HashMismatch,
                "artifact content hash does not match expected hash",
            ));
        }
    }

    Ok(ArtifactInfo {
        content_type: None,
        size: content.len(),
        hash,
    })
}
