//! Artifact storage adapters.

use crate::config::{PalaceConfig, StorageBackend};
use crate::error::{PalaceError, PalaceErrorCode, PalaceResult};
use crate::models::TrustInfo;
use sha2::{Digest, Sha256};
use std::net::IpAddr;
use url::Url;

/// Metadata about a fetched artifact.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ArtifactInfo {
    pub content_type: Option<String>,
    pub size: usize,
    pub hash: String,
}

#[cfg(feature = "reqwest")]
struct FetchedArtifact {
    content: Vec<u8>,
    content_type: Option<String>,
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

    if host.parse::<IpAddr>().is_ok_and(is_unsafe_artifact_address) {
        return Err(PalaceError::new(
            PalaceErrorCode::ArtifactNotAllowed,
            "artifact URL must not target a loopback, private, or local network address",
        ));
    }

    let allowed: &[String] = if config.storage.allowed_hosts.is_empty() {
        &[
            "github.com".to_string(),
            "objects.githubusercontent.com".to_string(),
        ]
    } else {
        &config.storage.allowed_hosts
    };

    if !allowed.iter().any(|allowed_host| host == allowed_host) {
        return Err(PalaceError::new(
            PalaceErrorCode::ArtifactNotAllowed,
            format!("host '{host}' is not in the allowed artifact host list"),
        ));
    }

    Ok(())
}

fn is_unsafe_artifact_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => {
            address.is_loopback()
                || address.is_private()
                || address.is_link_local()
                || address.is_unspecified()
                || address.is_multicast()
                || address.is_broadcast()
        }
        IpAddr::V6(address) => {
            address.is_loopback()
                || address.is_unspecified()
                || address.is_unique_local()
                || address.is_unicast_link_local()
                || address.is_multicast()
        }
    }
}

/// Compute the SHA-256 hash of artifact content.
pub fn compute_content_hash(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    hex::encode(hasher.finalize())
}

/// Verify fetched artifact bytes against the package trust metadata.
pub fn verify_artifact_content(
    content: &[u8],
    content_type: Option<String>,
    trust: &TrustInfo,
) -> PalaceResult<ArtifactInfo> {
    crate::trust::verify_content_hash(content, trust)?;

    match (&trust.signature, &trust.public_key) {
        (None, None) => {}
        (Some(_), Some(_)) => crate::trust::verify_signature(trust, content)?,
        _ => {
            return Err(PalaceError::new(
                PalaceErrorCode::SignatureInvalid,
                "signature and public_key must be provided together",
            ));
        }
    }

    Ok(ArtifactInfo {
        content_type,
        size: content.len(),
        hash: compute_content_hash(content),
    })
}
/// Fetch artifact content with redirect, destination, and size enforcement.
#[cfg(feature = "reqwest")]
pub async fn fetch_with_redirect_limit(url: &str, max_redirects: u32) -> PalaceResult<Vec<u8>> {
    let mut config = PalaceConfig::default();
    config.registry.max_redirects = max_redirects;
    Ok(fetch_artifact(url, &config).await?.content)
}

#[cfg(feature = "reqwest")]
async fn fetch_artifact(url: &str, config: &PalaceConfig) -> PalaceResult<FetchedArtifact> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| {
            PalaceError::new(
                PalaceErrorCode::StorageError,
                format!("failed to build HTTP client: {error}"),
            )
        })?;
    let mut next_url = Url::parse(url)
        .map_err(|_| PalaceError::new(PalaceErrorCode::BadRequest, "invalid artifact URL"))?;

    for redirect_count in 0..=config.registry.max_redirects {
        validate_artifact_destination(&next_url, config).await?;

        let mut response = client.get(next_url.clone()).send().await.map_err(|error| {
            PalaceError::new(
                PalaceErrorCode::StorageError,
                format!("failed to fetch artifact: {error}"),
            )
        })?;

        if response.status().is_redirection() {
            if redirect_count == config.registry.max_redirects {
                return Err(PalaceError::new(
                    PalaceErrorCode::BadRequest,
                    "artifact redirect limit exceeded",
                ));
            }

            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .ok_or_else(|| {
                    PalaceError::new(
                        PalaceErrorCode::BadRequest,
                        "artifact redirect is missing a location",
                    )
                })?
                .to_str()
                .map_err(|_| {
                    PalaceError::new(
                        PalaceErrorCode::BadRequest,
                        "artifact redirect location is invalid",
                    )
                })?;
            next_url = next_url.join(location).map_err(|_| {
                PalaceError::new(
                    PalaceErrorCode::BadRequest,
                    "artifact redirect location is invalid",
                )
            })?;
            continue;
        }

        if !response.status().is_success() {
            return Err(PalaceError::new(
                PalaceErrorCode::StorageError,
                format!("artifact fetch failed: HTTP {}", response.status()),
            ));
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);
        validate_content_type(content_type.as_deref())?;

        if response
            .content_length()
            .is_some_and(|size| size > config.storage.max_artifact_size_bytes as u64)
        {
            return Err(PalaceError::new(
                PalaceErrorCode::TooLarge,
                "artifact exceeds the configured maximum size",
            ));
        }

        let mut content = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|error| {
            PalaceError::new(
                PalaceErrorCode::StorageError,
                format!("failed to read artifact bytes: {error}"),
            )
        })? {
            if content.len().saturating_add(chunk.len()) > config.storage.max_artifact_size_bytes {
                return Err(PalaceError::new(
                    PalaceErrorCode::TooLarge,
                    "artifact exceeds the configured maximum size",
                ));
            }
            content.extend_from_slice(&chunk);
        }

        return Ok(FetchedArtifact {
            content,
            content_type,
        });
    }

    unreachable!("redirect loop exits with a result")
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
    let mut config = PalaceConfig::default();
    config.registry.max_redirects = max_redirects;
    fetch_and_verify_with_config(url, expected_hash, &config).await
}

/// Fetch artifact metadata and verify its content hash using registry configuration.
#[cfg(feature = "reqwest")]
pub async fn fetch_and_verify_with_config(
    url: &str,
    expected_hash: Option<&str>,
    config: &PalaceConfig,
) -> PalaceResult<ArtifactInfo> {
    let artifact = fetch_artifact(url, config).await?;
    let hash = compute_content_hash(&artifact.content);

    if let Some(expected_hash) = expected_hash {
        if hash != expected_hash {
            return Err(PalaceError::new(
                PalaceErrorCode::HashMismatch,
                "artifact content hash does not match expected hash",
            ));
        }
    }

    Ok(ArtifactInfo {
        content_type: artifact.content_type,
        size: artifact.content.len(),
        hash,
    })
}

/// Fetch artifact metadata and verify its content hash (stub without reqwest).
#[cfg(not(feature = "reqwest"))]
pub async fn fetch_and_verify_with_config(
    _url: &str,
    _expected_hash: Option<&str>,
    _config: &PalaceConfig,
) -> PalaceResult<ArtifactInfo> {
    Err(PalaceError::new(
        PalaceErrorCode::NotImplemented,
        "reqwest feature not enabled",
    ))
}

/// Fetch an artifact and verify the declared package digest and signature.
#[cfg(feature = "reqwest")]
pub async fn fetch_and_verify_package_artifact(
    url: &str,
    trust: &TrustInfo,
    config: &PalaceConfig,
) -> PalaceResult<ArtifactInfo> {
    let artifact = fetch_artifact(url, config).await?;
    verify_artifact_content(&artifact.content, artifact.content_type, trust)
}

/// Fetch-and-verify stub when HTTP fetching is not enabled.
#[cfg(not(feature = "reqwest"))]
pub async fn fetch_and_verify_package_artifact(
    _url: &str,
    _trust: &TrustInfo,
    _config: &PalaceConfig,
) -> PalaceResult<ArtifactInfo> {
    Err(PalaceError::new(
        PalaceErrorCode::NotImplemented,
        "reqwest feature not enabled",
    ))
}

#[cfg(feature = "reqwest")]
async fn validate_artifact_destination(url: &Url, config: &PalaceConfig) -> PalaceResult<()> {
    validate_artifact_url(url.as_str(), config)?;

    let host = url.host_str().ok_or_else(|| {
        PalaceError::new(
            PalaceErrorCode::BadRequest,
            "artifact URL is missing a host",
        )
    })?;
    let port = url.port_or_known_default().unwrap_or(443);
    let addresses = tokio::net::lookup_host((host, port))
        .await
        .map_err(|error| {
            PalaceError::new(
                PalaceErrorCode::StorageError,
                format!("failed to resolve artifact host: {error}"),
            )
        })?;

    for address in addresses {
        if is_unsafe_artifact_address(address.ip()) {
            return Err(PalaceError::new(
                PalaceErrorCode::ArtifactNotAllowed,
                "artifact host resolves to a loopback, private, or local network address",
            ));
        }
    }

    Ok(())
}

#[cfg(feature = "reqwest")]
fn validate_content_type(content_type: Option<&str>) -> PalaceResult<()> {
    if let Some(content_type) = content_type {
        if !content_type.starts_with("application/")
            && !content_type.starts_with("binary/")
            && !content_type.starts_with("text/")
            && !content_type.is_empty()
        {
            return Err(PalaceError::new(
                PalaceErrorCode::BadRequest,
                format!("unallowed content type for artifact: {content_type}"),
            ));
        }
    }

    Ok(())
}
