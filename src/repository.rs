//! Repository abstraction for persistence.

pub mod memory;
#[cfg(feature = "postgres")]
pub mod postgres;

use crate::error::PalaceResult;
use crate::models::*;
use crate::pagination::Pagination;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct VerifiedArtifact {
    pub url: String,
    pub content_type: Option<String>,
    pub size_bytes: i64,
    pub content_hash: String,
    pub signature: Option<String>,
    pub public_key: Option<String>,
}

/// Repository backend variants.
#[derive(Debug, Clone)]
pub enum PackageRepository {
    Memory(memory::InMemoryRepository),
    #[cfg(feature = "postgres")]
    Postgres(postgres::PostgresRepository),
}

pub(crate) fn token_id_from_opaque(token: &str) -> Option<Uuid> {
    let value = token.strip_prefix("kop_")?;
    let (id, secret) = value.split_once('_')?;
    if secret.len() != 32 || !secret.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Uuid::parse_str(id).ok()
}

macro_rules! dispatch {
    ($self:expr, $method:ident, $($arg:expr),*) => {
        match $self {
            Self::Memory(r) => r.$method($($arg),*).await,
            #[cfg(feature = "postgres")]
            Self::Postgres(r) => r.$method($($arg),*).await,
        }
    };
}

impl PackageRepository {
    pub async fn is_healthy(&self) -> bool {
        dispatch!(self, is_healthy,)
    }

    pub async fn create_publisher(&self, publisher: &Publisher) -> PalaceResult<Publisher> {
        dispatch!(self, create_publisher, publisher)
    }

    pub async fn get_publisher_by_id(&self, id: Uuid) -> PalaceResult<Publisher> {
        dispatch!(self, get_publisher_by_id, id)
    }

    pub async fn get_publisher_by_name(&self, name: &str) -> PalaceResult<Publisher> {
        dispatch!(self, get_publisher_by_name, name)
    }

    pub async fn get_publisher_verification(
        &self,
        publisher_id: Uuid,
    ) -> PalaceResult<PublisherVerification> {
        dispatch!(self, get_publisher_verification, publisher_id)
    }

    pub async fn set_publisher_verification(
        &self,
        verification: &PublisherVerification,
    ) -> PalaceResult<PublisherVerification> {
        dispatch!(self, set_publisher_verification, verification)
    }
    pub async fn list_publishers(&self) -> PalaceResult<Vec<Publisher>> {
        dispatch!(self, list_publishers,)
    }

    pub async fn update_publisher_role(&self, id: Uuid, role: Role) -> PalaceResult<Publisher> {
        dispatch!(self, update_publisher_role, id, role)
    }

    pub async fn create_api_token(&self, token: &ApiToken) -> PalaceResult<()> {
        dispatch!(self, create_api_token, token)
    }

    pub async fn create_api_token_with_audit(
        &self,
        token: &ApiToken,
        event: &AuditEvent,
    ) -> PalaceResult<()> {
        dispatch!(self, create_api_token_with_audit, token, event)
    }

    pub async fn get_api_token_by_plaintext(&self, plaintext: &str) -> PalaceResult<ApiToken> {
        dispatch!(self, get_api_token_by_plaintext, plaintext)
    }
    pub async fn get_api_token_by_id(&self, id: Uuid) -> PalaceResult<ApiToken> {
        dispatch!(self, get_api_token_by_id, id)
    }

    pub async fn revoke_api_token(&self, id: Uuid) -> PalaceResult<()> {
        dispatch!(self, revoke_api_token, id)
    }

    pub async fn revoke_api_token_with_audit(
        &self,
        id: Uuid,
        event: &AuditEvent,
    ) -> PalaceResult<()> {
        dispatch!(self, revoke_api_token_with_audit, id, event)
    }
    pub async fn touch_api_token(&self, id: Uuid) -> PalaceResult<()> {
        dispatch!(self, touch_api_token, id)
    }

    pub async fn list_api_tokens(&self, publisher_id: Uuid) -> PalaceResult<Vec<ApiToken>> {
        dispatch!(self, list_api_tokens, publisher_id)
    }

    pub async fn list_packages(
        &self,
        filters: PackageFilters,
        pagination: Pagination,
    ) -> PalaceResult<(usize, Vec<Package>)> {
        dispatch!(self, list_packages, filters, pagination)
    }

    pub async fn get_package(&self, id: &str) -> PalaceResult<Package> {
        dispatch!(self, get_package, id)
    }

    pub async fn get_package_publisher_id(&self, id: &str) -> PalaceResult<Option<Uuid>> {
        dispatch!(self, get_package_publisher_id, id)
    }

    pub async fn get_package_version(&self, id: &str, version: &str) -> PalaceResult<Package> {
        dispatch!(self, get_package_version, id, version)
    }

    pub async fn list_versions(&self, id: &str) -> PalaceResult<Vec<VersionInfo>> {
        dispatch!(self, list_versions, id)
    }

    pub async fn publish_package(&self, package: &Package) -> PalaceResult<Package> {
        dispatch!(self, publish_package, package)
    }

    pub async fn publish_verified_package(
        &self,
        package: &Package,
        artifact: Option<&VerifiedArtifact>,
        actor_id: Option<Uuid>,
    ) -> PalaceResult<Package> {
        dispatch!(self, publish_verified_package, package, artifact, actor_id)
    }

    pub async fn update_package(&self, package: &Package) -> PalaceResult<Package> {
        dispatch!(self, update_package, package)
    }

    pub async fn delete_package(&self, id: &str, publisher_id: Uuid) -> PalaceResult<()> {
        dispatch!(self, delete_package, id, publisher_id)
    }

    pub async fn record_download(&self, id: &str, version: &str) -> PalaceResult<()> {
        dispatch!(self, record_download, id, version)
    }

    pub async fn record_download_with_context(
        &self,
        id: &str,
        version: &str,
        dedupe_key: Option<&str>,
    ) -> PalaceResult<bool> {
        dispatch!(self, record_download_with_context, id, version, dedupe_key)
    }

    pub async fn search(
        &self,
        query: &str,
        pagination: Pagination,
    ) -> PalaceResult<(usize, Vec<Package>)> {
        dispatch!(self, search, query, pagination)
    }

    pub async fn featured(&self, limit: usize) -> PalaceResult<Vec<Package>> {
        dispatch!(self, featured, limit)
    }

    pub async fn trending(&self, limit: usize) -> PalaceResult<Vec<Package>> {
        dispatch!(self, trending, limit)
    }

    pub async fn newest(&self, limit: usize) -> PalaceResult<Vec<Package>> {
        dispatch!(self, newest, limit)
    }

    pub async fn categories(&self) -> PalaceResult<Vec<String>> {
        dispatch!(self, categories,)
    }

    pub async fn runtimes(&self) -> PalaceResult<Vec<String>> {
        dispatch!(self, runtimes,)
    }

    pub async fn add_review(&self, review: &Review) -> PalaceResult<Review> {
        dispatch!(self, add_review, review)
    }

    pub async fn list_reviews(&self, package_id: &str) -> PalaceResult<Vec<Review>> {
        dispatch!(self, list_reviews, package_id)
    }

    pub async fn moderate_review(
        &self,
        package_id: &str,
        review_id: Uuid,
        status: ReviewStatus,
        moderator_id: Uuid,
        reason: Option<String>,
    ) -> PalaceResult<Review> {
        dispatch!(
            self,
            moderate_review,
            package_id,
            review_id,
            status,
            moderator_id,
            reason
        )
    }

    pub async fn moderate_review_with_audit(
        &self,
        package_id: &str,
        review_id: Uuid,
        status: ReviewStatus,
        moderator_id: Uuid,
        reason: Option<String>,
        event: &AuditEvent,
    ) -> PalaceResult<Review> {
        dispatch!(
            self,
            moderate_review_with_audit,
            package_id,
            review_id,
            status,
            moderator_id,
            reason,
            event
        )
    }

    pub async fn record_trust_transition(&self, transition: &TrustTransition) -> PalaceResult<()> {
        dispatch!(self, record_trust_transition, transition)
    }

    pub async fn list_trust_transitions(
        &self,
        package_id: &str,
    ) -> PalaceResult<Vec<TrustTransition>> {
        dispatch!(self, list_trust_transitions, package_id)
    }

    pub async fn record_audit_event(&self, event: &AuditEvent) -> PalaceResult<()> {
        dispatch!(self, record_audit_event, event)
    }

    /// Yank a package version (mark as not installable without deleting).
    pub async fn yank_package(&self, id: &str, version: &str) -> PalaceResult<()> {
        dispatch!(self, yank_package, id, version)
    }

    /// Unyank a package version (restore installability).
    pub async fn unyank_package(&self, id: &str, version: &str) -> PalaceResult<()> {
        dispatch!(self, unyank_package, id, version)
    }
}

#[derive(Debug, Default, Clone)]
pub struct PackageFilters {
    pub q: Option<String>,
    pub kind: Option<String>,
    pub category: Option<String>,
    pub runtime: Option<String>,
}
