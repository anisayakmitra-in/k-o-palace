//! Artifact storage adapters.

use crate::config::{PalaceConfig, StorageBackend};
use crate::error::{PalaceError, PalaceErrorCode, PalaceResult};
use crate::models::TrustInfo;
use sha2::{Digest, Sha256};
use std::net::IpAddr;
#[cfg(feature = "reqwest")]
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, OnceLock,
    },
};
#[cfg(feature = "reqwest")]
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
#[cfg(feature = "reqwest")]
use tokio_util::io::ReaderStream;
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

#[cfg(feature = "reqwest")]
const MAX_CONCURRENT_ARTIFACT_DOWNLOADS: usize = 8;
#[cfg(feature = "reqwest")]
const MAX_AGGREGATE_RETAINED_ARTIFACT_BYTES: usize = 1024 * 1024 * 1024;

#[cfg(feature = "reqwest")]
#[derive(Debug)]
struct ArtifactResourceBudget {
    state: Arc<ArtifactResourceState>,
}

#[cfg(feature = "reqwest")]
#[derive(Debug)]
struct ArtifactResourceState {
    max_concurrent_downloads: usize,
    max_retained_bytes: usize,
    active_downloads: AtomicUsize,
    reserved_bytes: AtomicUsize,
}

#[cfg(feature = "reqwest")]
#[derive(Debug)]
struct ArtifactResourcePermit {
    state: Arc<ArtifactResourceState>,
    reserved_bytes: usize,
}

#[cfg(feature = "reqwest")]
impl ArtifactResourceBudget {
    fn new(max_concurrent_downloads: usize, max_retained_bytes: usize) -> PalaceResult<Self> {
        if max_concurrent_downloads == 0 || max_retained_bytes == 0 {
            return Err(PalaceError::new(
                PalaceErrorCode::BadRequest,
                "artifact resource limits must be greater than zero",
            ));
        }

        Ok(Self {
            state: Arc::new(ArtifactResourceState {
                max_concurrent_downloads,
                max_retained_bytes,
                active_downloads: AtomicUsize::new(0),
                reserved_bytes: AtomicUsize::new(0),
            }),
        })
    }

    fn try_acquire(&self, reserved_bytes: usize) -> PalaceResult<ArtifactResourcePermit> {
        if reserved_bytes > self.state.max_retained_bytes {
            return Err(resource_budget_exhausted());
        }

        if self
            .state
            .active_downloads
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
                (active < self.state.max_concurrent_downloads).then_some(active + 1)
            })
            .is_err()
        {
            return Err(resource_budget_exhausted());
        }

        let reserved = self.state.reserved_bytes.fetch_update(
            Ordering::AcqRel,
            Ordering::Acquire,
            |current| {
                current
                    .checked_add(reserved_bytes)
                    .filter(|next| *next <= self.state.max_retained_bytes)
            },
        );
        if reserved.is_err() {
            self.state.active_downloads.fetch_sub(1, Ordering::AcqRel);
            return Err(resource_budget_exhausted());
        }

        Ok(ArtifactResourcePermit {
            state: Arc::clone(&self.state),
            reserved_bytes,
        })
    }

    #[cfg(test)]
    fn active_downloads(&self) -> usize {
        self.state.active_downloads.load(Ordering::Acquire)
    }

    #[cfg(test)]
    fn reserved_bytes(&self) -> usize {
        self.state.reserved_bytes.load(Ordering::Acquire)
    }
}

#[cfg(feature = "reqwest")]
impl Drop for ArtifactResourcePermit {
    fn drop(&mut self) {
        self.state
            .reserved_bytes
            .fetch_sub(self.reserved_bytes, Ordering::AcqRel);
        self.state.active_downloads.fetch_sub(1, Ordering::AcqRel);
    }
}

#[cfg(feature = "reqwest")]
fn resource_budget_exhausted() -> PalaceError {
    PalaceError::new(
        PalaceErrorCode::RateLimited,
        "artifact download resource budget exhausted",
    )
}

#[cfg(feature = "reqwest")]
fn acquire_artifact_resources(reserved_bytes: usize) -> PalaceResult<ArtifactResourcePermit> {
    static BUDGET: OnceLock<ArtifactResourceBudget> = OnceLock::new();
    BUDGET
        .get_or_init(|| {
            ArtifactResourceBudget::new(
                MAX_CONCURRENT_ARTIFACT_DOWNLOADS,
                MAX_AGGREGATE_RETAINED_ARTIFACT_BYTES,
            )
            .expect("static artifact resource limits are valid")
        })
        .try_acquire(reserved_bytes)
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
    Unsupported(StorageBackend),
}

impl ArtifactStorage {
    /// Create a storage adapter from the configuration.
    pub fn from_config(config: &PalaceConfig) -> Self {
        match config.storage.backend {
            StorageBackend::Local => ArtifactStorage::Local(LocalFileStorage),
            StorageBackend::GitHub => ArtifactStorage::GitHub(GitHubReleaseStorage),
            backend => ArtifactStorage::Unsupported(backend),
        }
    }

    /// Reject configured backends that do not have an implementation yet.
    pub fn ensure_supported(&self) -> PalaceResult<()> {
        if let Self::Unsupported(backend) = self {
            return Err(PalaceError::new(
                PalaceErrorCode::NotImplemented,
                format!(
                    "storage backend '{}' is not implemented; use local or github",
                    backend_name(backend)
                ),
            ));
        }
        Ok(())
    }
}

fn backend_name(backend: &StorageBackend) -> &'static str {
    match backend {
        StorageBackend::Local => "local",
        StorageBackend::GitHub => "github",
        StorageBackend::GitLab => "gitlab",
        StorageBackend::Codeberg => "codeberg",
        StorageBackend::Oci => "oci",
        StorageBackend::S3 => "s3",
        StorageBackend::Azure => "azure",
        StorageBackend::Gcs => "gcs",
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
    let _resource_permit = acquire_artifact_resources(config.storage.max_artifact_size_bytes)?;
    let mut next_url = Url::parse(url)
        .map_err(|_| PalaceError::new(PalaceErrorCode::BadRequest, "invalid artifact URL"))?;

    for redirect_count in 0..=config.registry.max_redirects {
        let mut response = send_pinned_request(&next_url, config).await?;

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

#[cfg(feature = "reqwest")]
struct TemporaryArtifact {
    path: std::path::PathBuf,
}

#[cfg(feature = "reqwest")]
impl Drop for TemporaryArtifact {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(feature = "reqwest")]
async fn fetch_and_verify_package_artifact_file(
    url: &str,
    trust: &TrustInfo,
    config: &PalaceConfig,
) -> PalaceResult<VerifiedArtifactFile> {
    let resource_permit = acquire_artifact_resources(config.storage.max_artifact_size_bytes)?;
    let mut next_url = Url::parse(url)
        .map_err(|_| PalaceError::new(PalaceErrorCode::BadRequest, "invalid artifact URL"))?;

    let mut redirect_count = 0u32;
    let (content_type, size, hash, mut file, temporary) = loop {
        let response = send_pinned_request(&next_url, config).await?;

        if response.status().is_redirection() {
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
            if redirect_count >= config.registry.max_redirects {
                return Err(PalaceError::new(
                    PalaceErrorCode::BadRequest,
                    "artifact redirect limit exceeded",
                ));
            }
            redirect_count += 1;
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
            .is_some_and(|value| value > config.storage.max_artifact_size_bytes as u64)
        {
            return Err(PalaceError::new(
                PalaceErrorCode::TooLarge,
                "artifact exceeds the configured maximum size",
            ));
        }

        let (temporary, mut file) = create_temporary_artifact().await?;
        let mut hasher = Sha256::new();
        let mut size = 0usize;
        let mut response = response;
        while let Some(chunk) = response.chunk().await.map_err(|error| {
            PalaceError::new(
                PalaceErrorCode::StorageError,
                format!("failed to read artifact bytes: {error}"),
            )
        })? {
            if size.saturating_add(chunk.len()) > config.storage.max_artifact_size_bytes {
                return Err(PalaceError::new(
                    PalaceErrorCode::TooLarge,
                    "artifact exceeds the configured maximum size",
                ));
            }
            hasher.update(&chunk);
            file.write_all(&chunk).await.map_err(|error| {
                PalaceError::new(
                    PalaceErrorCode::StorageError,
                    format!("failed to write temporary artifact: {error}"),
                )
            })?;
            size += chunk.len();
        }
        file.flush().await.map_err(|error| {
            PalaceError::new(
                PalaceErrorCode::StorageError,
                format!("failed to flush temporary artifact: {error}"),
            )
        })?;
        break (
            content_type,
            size,
            hex::encode(hasher.finalize()),
            file,
            temporary,
        );
    };

    let expected = trust
        .content_hash
        .as_deref()
        .and_then(|value| value.strip_prefix("sha256:"))
        .ok_or_else(|| {
            PalaceError::new(
                PalaceErrorCode::HashMismatch,
                "missing or invalid content_hash",
            )
        })?;
    if hash != expected {
        return Err(PalaceError::new(
            PalaceErrorCode::HashMismatch,
            "artifact content hash does not match expected hash",
        ));
    }

    match (&trust.signature, &trust.public_key) {
        (None, None) => {}
        (Some(_), Some(_)) => {
            if size > config.storage.max_signed_artifact_size_bytes {
                return Err(PalaceError::new(
                    PalaceErrorCode::TooLarge,
                    "signed artifact exceeds the configured in-memory verification limit",
                ));
            }
            file.seek(std::io::SeekFrom::Start(0))
                .await
                .map_err(|error| {
                    PalaceError::new(
                        PalaceErrorCode::StorageError,
                        format!("failed to seek temporary artifact: {error}"),
                    )
                })?;
            let mut content = Vec::with_capacity(size);
            file.read_to_end(&mut content).await.map_err(|error| {
                PalaceError::new(
                    PalaceErrorCode::StorageError,
                    format!("failed to read temporary artifact: {error}"),
                )
            })?;
            crate::trust::verify_signature(trust, &content)?;
        }
        _ => {
            return Err(PalaceError::new(
                PalaceErrorCode::SignatureInvalid,
                "signature and public_key must be provided together",
            ));
        }
    }

    file.seek(std::io::SeekFrom::Start(0))
        .await
        .map_err(|error| {
            PalaceError::new(
                PalaceErrorCode::StorageError,
                format!("failed to seek temporary artifact: {error}"),
            )
        })?;

    Ok(VerifiedArtifactFile {
        content_type,
        size,
        hash,
        file,
        temporary,
        resource_permit,
    })
}
#[cfg(feature = "reqwest")]
struct VerifiedArtifactFile {
    content_type: Option<String>,
    size: usize,
    hash: String,
    file: tokio::fs::File,
    temporary: TemporaryArtifact,
    resource_permit: ArtifactResourcePermit,
}

#[cfg(feature = "reqwest")]
pub struct VerifiedArtifactStream {
    inner: ReaderStream<tokio::fs::File>,
    _temporary: TemporaryArtifact,
    _resource_permit: ArtifactResourcePermit,
}

#[cfg(feature = "reqwest")]
impl Unpin for VerifiedArtifactStream {}

#[cfg(feature = "reqwest")]
impl futures_core::Stream for VerifiedArtifactStream {
    type Item = Result<bytes::Bytes, std::io::Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        std::pin::Pin::new(&mut self.get_mut().inner).poll_next(cx)
    }
}

#[cfg(feature = "reqwest")]
async fn fetch_and_verify_package_artifact_streaming(
    url: &str,
    trust: &TrustInfo,
    config: &PalaceConfig,
) -> PalaceResult<ArtifactInfo> {
    let artifact = fetch_and_verify_package_artifact_file(url, trust, config).await?;
    Ok(ArtifactInfo {
        content_type: artifact.content_type,
        size: artifact.size,
        hash: artifact.hash,
    })
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

#[cfg(feature = "reqwest")]
pub async fn fetch_and_verify_package_artifact_stream(
    url: &str,
    trust: &TrustInfo,
    config: &PalaceConfig,
) -> PalaceResult<(VerifiedArtifactStream, Option<String>, usize)> {
    let artifact = fetch_and_verify_package_artifact_file(url, trust, config).await?;
    Ok((
        VerifiedArtifactStream {
            inner: ReaderStream::new(artifact.file),
            _temporary: artifact.temporary,
            _resource_permit: artifact.resource_permit,
        },
        artifact.content_type,
        artifact.size,
    ))
}
/// Fetch an artifact and verify the declared package digest and signature.
#[cfg(feature = "reqwest")]
pub async fn fetch_and_verify_package_artifact(
    url: &str,
    trust: &TrustInfo,
    config: &PalaceConfig,
) -> PalaceResult<ArtifactInfo> {
    fetch_and_verify_package_artifact_streaming(url, trust, config).await
}

/// Fetch and verify an artifact before returning the exact verified bytes.
#[cfg(feature = "reqwest")]
pub async fn fetch_and_verify_package_artifact_content(
    url: &str,
    trust: &TrustInfo,
    config: &PalaceConfig,
) -> PalaceResult<(Vec<u8>, Option<String>)> {
    let artifact = fetch_artifact(url, config).await?;
    verify_artifact_content(&artifact.content, artifact.content_type.clone(), trust)?;
    Ok((artifact.content, artifact.content_type))
}

/// Fetch-and-verify content stub when HTTP fetching is not enabled.
#[cfg(not(feature = "reqwest"))]
pub async fn fetch_and_verify_package_artifact_content(
    _url: &str,
    _trust: &TrustInfo,
    _config: &PalaceConfig,
) -> PalaceResult<(Vec<u8>, Option<String>)> {
    Err(PalaceError::new(
        PalaceErrorCode::NotImplemented,
        "reqwest feature not enabled",
    ))
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
#[derive(Debug)]
struct ResolvedArtifactDestination {
    url: Url,
    host: String,
    addresses: Vec<SocketAddr>,
}

#[cfg(feature = "reqwest")]
impl ResolvedArtifactDestination {
    fn new(url: Url, addresses: Vec<SocketAddr>) -> PalaceResult<Self> {
        let host = url
            .host_str()
            .ok_or_else(|| {
                PalaceError::new(
                    PalaceErrorCode::BadRequest,
                    "artifact URL is missing a host",
                )
            })?
            .to_owned();

        if addresses.is_empty() {
            return Err(PalaceError::new(
                PalaceErrorCode::StorageError,
                "artifact host did not resolve to an address",
            ));
        }
        if addresses
            .iter()
            .any(|address| is_unsafe_artifact_address(address.ip()))
        {
            return Err(PalaceError::new(
                PalaceErrorCode::ArtifactNotAllowed,
                "artifact host resolves to a loopback, private, or local network address",
            ));
        }

        Ok(Self {
            url,
            host,
            addresses,
        })
    }

    fn url(&self) -> &Url {
        &self.url
    }

    fn host(&self) -> &str {
        &self.host
    }

    fn addresses(&self) -> &[SocketAddr] {
        &self.addresses
    }
}

#[cfg(feature = "reqwest")]
async fn resolve_artifact_destination(
    url: &Url,
    config: &PalaceConfig,
) -> PalaceResult<ResolvedArtifactDestination> {
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
        })?
        .collect();

    ResolvedArtifactDestination::new(url.clone(), addresses)
}

#[cfg(feature = "reqwest")]
fn build_pinned_client(destination: &ResolvedArtifactDestination) -> PalaceResult<reqwest::Client> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .resolve_to_addrs(destination.host(), destination.addresses())
        .build()
        .map_err(|error| {
            PalaceError::new(
                PalaceErrorCode::StorageError,
                format!("failed to build pinned HTTP client: {error}"),
            )
        })
}

#[cfg(feature = "reqwest")]
async fn send_pinned_request(url: &Url, config: &PalaceConfig) -> PalaceResult<reqwest::Response> {
    let destination = resolve_artifact_destination(url, config).await?;
    build_pinned_client(&destination)?
        .get(destination.url().clone())
        .send()
        .await
        .map_err(|error| {
            PalaceError::new(
                PalaceErrorCode::StorageError,
                format!("failed to fetch artifact: {error}"),
            )
        })
}

#[cfg(feature = "reqwest")]
async fn create_temporary_artifact() -> PalaceResult<(TemporaryArtifact, tokio::fs::File)> {
    let temporary = TemporaryArtifact {
        path: std::env::temp_dir().join(format!(
            "k-o-palace-artifact-{}.tmp",
            uuid::Uuid::new_v4().simple()
        )),
    };
    let mut options = tokio::fs::OpenOptions::new();
    options.read(true).write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    #[cfg(windows)]
    options.share_mode(0);

    let file = options.open(&temporary.path).await.map_err(|error| {
        PalaceError::new(
            PalaceErrorCode::StorageError,
            format!("failed to create temporary artifact: {error}"),
        )
    })?;
    Ok((temporary, file))
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

#[cfg(all(test, feature = "reqwest"))]
mod security_tests {
    use super::*;
    use std::net::{Ipv4Addr, SocketAddr};

    #[test]
    fn aggregate_budget_rejects_excess_concurrency_and_retained_bytes() {
        let budget = ArtifactResourceBudget::new(2, 10).expect("valid test budget");

        let first = budget.try_acquire(6).expect("first reservation");
        assert_eq!(budget.active_downloads(), 1);
        assert_eq!(budget.reserved_bytes(), 6);

        let error = budget.try_acquire(5).unwrap_err();
        assert_eq!(error.code, PalaceErrorCode::RateLimited);

        let second = budget.try_acquire(4).expect("remaining reservation");
        let error = budget.try_acquire(0).unwrap_err();
        assert_eq!(error.code, PalaceErrorCode::RateLimited);

        drop(first);
        assert_eq!(budget.active_downloads(), 1);
        assert_eq!(budget.reserved_bytes(), 4);

        drop(second);
        assert_eq!(budget.active_downloads(), 0);
        assert_eq!(budget.reserved_bytes(), 0);
    }

    #[test]
    fn pinned_destination_retains_the_approved_addresses_and_https_host() {
        let url = Url::parse("https://packages.example.test/release.tar.gz").unwrap();
        let addresses = vec![SocketAddr::new(Ipv4Addr::new(93, 184, 216, 34).into(), 443)];
        let destination = ResolvedArtifactDestination::new(url.clone(), addresses.clone())
            .expect("valid resolved destination");

        assert_eq!(destination.url(), &url);
        assert_eq!(destination.host(), "packages.example.test");
        assert_eq!(destination.addresses(), addresses.as_slice());
        build_pinned_client(&destination).expect("approved addresses build a pinned client");
    }

    #[tokio::test]
    async fn temporary_artifact_is_private_and_removed_on_drop() {
        let (temporary, file) = create_temporary_artifact()
            .await
            .expect("temporary artifact");
        let path = temporary.path.clone();
        assert!(path.exists());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }

        #[cfg(windows)]
        assert!(std::fs::File::open(&path).is_err());

        drop(file);
        drop(temporary);
        assert!(!path.exists());
    }
}
