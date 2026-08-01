//! In-memory repository implementation for tests and development.

use crate::error::{PalaceError, PalaceErrorCode, PalaceResult};
use crate::models::*;
use crate::pagination::Pagination;
use crate::repository::PackageFilters;
use chrono::{Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

type DownloadDedupeKey = (String, String, String);
type DownloadDedupeMap = HashMap<DownloadDedupeKey, chrono::DateTime<Utc>>;

#[derive(Debug, Default, Clone)]
pub struct InMemoryRepository {
    publishers: Arc<RwLock<HashMap<Uuid, Publisher>>>,
    verifications: Arc<RwLock<HashMap<Uuid, PublisherVerification>>>,
    tokens: Arc<RwLock<HashMap<Uuid, ApiToken>>>,
    last_used: Arc<RwLock<HashMap<Uuid, chrono::DateTime<Utc>>>>,
    packages: Arc<RwLock<HashMap<String, Vec<Package>>>>,
    package_owners: Arc<RwLock<HashMap<String, Uuid>>>,
    reviews: Arc<RwLock<HashMap<String, Vec<Review>>>>,
    transitions: Arc<RwLock<HashMap<String, Vec<TrustTransition>>>>,
    audit: Arc<RwLock<Vec<AuditEvent>>>,
    download_dedup: Arc<RwLock<DownloadDedupeMap>>,
    artifacts: Arc<RwLock<HashMap<(String, String), crate::repository::VerifiedArtifact>>>,
}

impl InMemoryRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed(&mut self) {
        // self seeding is handled by the app layer for now
    }

    async fn packages_filtered(&self, filters: PackageFilters) -> Vec<Package> {
        let pkgs = self.packages.read().await;
        let mut out: Vec<Package> = pkgs
            .values()
            .flat_map(|versions| versions.iter().max_by_key(|p| p.version.clone()).cloned())
            .collect();

        if let Some(kind) = filters.kind {
            out.retain(|p| p.kind.as_str() == kind);
        }
        if let Some(cat) = filters.category {
            out.retain(|p| p.tags.iter().any(|t| t.eq_ignore_ascii_case(&cat)));
        }
        if let Some(rt) = filters.runtime {
            out.retain(|p| p.compatibility.runtimes.iter().any(|r| r.contains(&rt)));
        }
        if let Some(q) = filters.q {
            let q = q.to_lowercase();
            out.retain(|p| {
                p.name.to_lowercase().contains(&q)
                    || p.id.to_lowercase().contains(&q)
                    || p.description.to_lowercase().contains(&q)
                    || p.tags.iter().any(|t| t.to_lowercase().contains(&q))
            });
        }
        out
    }
}

impl InMemoryRepository {
    pub async fn is_healthy(&self) -> bool {
        true
    }

    pub async fn create_publisher(&self, publisher: &Publisher) -> PalaceResult<Publisher> {
        let mut map = self.publishers.write().await;
        if map.values().any(|p| p.name == publisher.name) {
            return Err(PalaceError::new(
                PalaceErrorCode::Conflict,
                "publisher name already exists",
            ));
        }
        map.insert(publisher.id, publisher.clone());
        Ok(publisher.clone())
    }

    pub async fn get_publisher_by_id(&self, id: Uuid) -> PalaceResult<Publisher> {
        let map = self.publishers.read().await;
        map.get(&id)
            .cloned()
            .ok_or_else(|| PalaceError::new(PalaceErrorCode::NotFound, "publisher not found"))
    }

    pub async fn get_publisher_by_name(&self, name: &str) -> PalaceResult<Publisher> {
        let map = self.publishers.read().await;
        map.values()
            .find(|p| p.name == name)
            .cloned()
            .ok_or_else(|| PalaceError::new(PalaceErrorCode::NotFound, "publisher not found"))
    }

    pub async fn list_publishers(&self) -> PalaceResult<Vec<Publisher>> {
        let map = self.publishers.read().await;
        let mut publishers = map.values().cloned().collect::<Vec<_>>();
        publishers.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(publishers)
    }

    pub async fn update_publisher_role(&self, id: Uuid, role: Role) -> PalaceResult<Publisher> {
        let mut map = self.publishers.write().await;
        let publisher = map
            .get_mut(&id)
            .ok_or_else(|| PalaceError::new(PalaceErrorCode::NotFound, "publisher not found"))?;
        publisher.role = role;
        Ok(publisher.clone())
    }

    pub async fn get_publisher_verification(
        &self,
        publisher_id: Uuid,
    ) -> PalaceResult<PublisherVerification> {
        self.get_publisher_by_id(publisher_id).await?;
        let map = self.verifications.read().await;
        Ok(map
            .get(&publisher_id)
            .cloned()
            .unwrap_or(PublisherVerification {
                publisher_id,
                verified: false,
                verified_at: None,
                verified_by: None,
                reason: None,
            }))
    }

    pub async fn set_publisher_verification(
        &self,
        verification: &PublisherVerification,
    ) -> PalaceResult<PublisherVerification> {
        self.get_publisher_by_id(verification.publisher_id).await?;
        let mut map = self.verifications.write().await;
        map.insert(verification.publisher_id, verification.clone());
        Ok(verification.clone())
    }
    pub async fn create_api_token(&self, token: &ApiToken) -> PalaceResult<()> {
        let mut map = self.tokens.write().await;
        map.insert(token.id, token.clone());
        Ok(())
    }

    pub async fn create_api_token_with_audit(
        &self,
        token: &ApiToken,
        event: &AuditEvent,
    ) -> PalaceResult<()> {
        self.create_api_token(token).await?;
        self.record_audit_event(event).await
    }

    pub async fn get_api_token_by_plaintext(&self, plaintext: &str) -> PalaceResult<ApiToken> {
        let id = crate::repository::token_id_from_opaque(plaintext).ok_or_else(|| {
            PalaceError::new(PalaceErrorCode::Unauthorized, "invalid or revoked token")
        })?;
        let token = self.get_api_token_by_id(id).await?;
        if bcrypt::verify(plaintext.as_bytes(), &token.token_hash).unwrap_or(false) {
            Ok(token)
        } else {
            Err(PalaceError::new(
                PalaceErrorCode::Unauthorized,
                "invalid or revoked token",
            ))
        }
    }

    pub async fn revoke_api_token(&self, id: Uuid) -> PalaceResult<()> {
        let mut map = self.tokens.write().await;
        let token = map
            .get_mut(&id)
            .ok_or_else(|| PalaceError::new(PalaceErrorCode::NotFound, "token not found"))?;
        token.revoked_at = Some(Utc::now());
        Ok(())
    }

    pub async fn revoke_api_token_with_audit(
        &self,
        id: Uuid,
        event: &AuditEvent,
    ) -> PalaceResult<()> {
        self.revoke_api_token(id).await?;
        self.record_audit_event(event).await
    }
    pub async fn get_api_token_by_id(&self, id: Uuid) -> PalaceResult<ApiToken> {
        let map = self.tokens.read().await;
        map.get(&id)
            .filter(|token| token.revoked_at.is_none())
            .cloned()
            .ok_or_else(|| {
                PalaceError::new(PalaceErrorCode::Unauthorized, "invalid or revoked token")
            })
    }

    pub async fn touch_api_token(&self, id: Uuid) -> PalaceResult<()> {
        let mut map = self.last_used.write().await;
        map.insert(id, Utc::now());
        Ok(())
    }

    pub async fn list_api_tokens(&self, publisher_id: Uuid) -> PalaceResult<Vec<ApiToken>> {
        let map = self.tokens.read().await;
        Ok(map
            .values()
            .filter(|t| t.publisher_id == publisher_id)
            .cloned()
            .collect())
    }

    pub async fn list_packages(
        &self,
        filters: PackageFilters,
        pagination: Pagination,
    ) -> PalaceResult<(usize, Vec<Package>)> {
        let mut pkgs = self.packages_filtered(filters).await;
        pkgs.sort_by_key(|b| std::cmp::Reverse(b.downloads));
        let total = pkgs.len();
        let (start, end) = pagination.bounds(total);
        Ok((total, pkgs[start..end].to_vec()))
    }

    pub async fn get_package(&self, id: &str) -> PalaceResult<Package> {
        let map = self.packages.read().await;
        let versions = map
            .get(id)
            .ok_or_else(|| PalaceError::new(PalaceErrorCode::NotFound, "package not found"))?;
        versions
            .iter()
            .max_by_key(|p| p.version.clone())
            .cloned()
            .ok_or_else(|| PalaceError::new(PalaceErrorCode::NotFound, "no versions found"))
    }
    pub async fn get_package_publisher_id(&self, id: &str) -> PalaceResult<Option<Uuid>> {
        let has_versions = self
            .packages
            .read()
            .await
            .get(id)
            .is_some_and(|versions| !versions.is_empty());
        if !has_versions {
            return Err(PalaceError::new(
                PalaceErrorCode::NotFound,
                "package not found",
            ));
        }
        Ok(self.package_owners.read().await.get(id).copied())
    }

    pub async fn get_package_version(&self, id: &str, version: &str) -> PalaceResult<Package> {
        let map = self.packages.read().await;
        let versions = map
            .get(id)
            .ok_or_else(|| PalaceError::new(PalaceErrorCode::NotFound, "package not found"))?;
        versions
            .iter()
            .find(|p| p.version == version)
            .cloned()
            .ok_or_else(|| PalaceError::new(PalaceErrorCode::NotFound, "version not found"))
    }

    pub async fn list_versions(&self, id: &str) -> PalaceResult<Vec<VersionInfo>> {
        let map = self.packages.read().await;
        let versions = map
            .get(id)
            .ok_or_else(|| PalaceError::new(PalaceErrorCode::NotFound, "package not found"))?;
        Ok(versions
            .iter()
            .map(|p| VersionInfo {
                version: p.version.clone(),
                created_at: p.created_at,
                artifact_url: p.artifact_url.clone(),
                content_hash: p.trust.content_hash.clone(),
            })
            .collect())
    }

    pub async fn publish_package(&self, package: &Package) -> PalaceResult<Package> {
        let actor_id = match self.get_publisher_by_name(&package.trust.publisher).await {
            Ok(publisher) => Some(publisher.id),
            Err(error) if error.code == PalaceErrorCode::NotFound => None,
            Err(error) => return Err(error),
        };
        self.publish_verified_package(package, None, actor_id).await
    }

    pub async fn publish_verified_package(
        &self,
        package: &Package,
        artifact: Option<&crate::repository::VerifiedArtifact>,
        actor_id: Option<Uuid>,
    ) -> PalaceResult<Package> {
        let actor_name = if let Some(actor_id) = actor_id {
            Some(self.get_publisher_by_id(actor_id).await?.name)
        } else {
            None
        };
        if let Some(actor_name) = actor_name.as_deref() {
            if package.id.contains('/') && crate::identity::namespace_of(&package.id) != actor_name
            {
                return Err(PalaceError::new(
                    PalaceErrorCode::Forbidden,
                    "cannot publish under another publisher namespace",
                ));
            }
        }
        {
            let mut owners = self.package_owners.write().await;
            let mut map = self.packages.write().await;
            let versions = map.entry(package.id.clone()).or_default();
            if versions.iter().any(|p| p.version == package.version) {
                return Err(PalaceError::new(
                    PalaceErrorCode::Conflict,
                    format!(
                        "version {} already exists for package {}",
                        package.version, package.id
                    ),
                ));
            }
            match actor_id {
                Some(actor_id) => {
                    if let Some(owner_id) = owners.get(&package.id) {
                        if *owner_id != actor_id {
                            return Err(PalaceError::new(
                                PalaceErrorCode::Forbidden,
                                "cannot publish another publisher's package",
                            ));
                        }
                    } else if !versions.is_empty() {
                        return Err(PalaceError::new(
                            PalaceErrorCode::Forbidden,
                            "package ownership is not assigned",
                        ));
                    }
                }
                None if !versions.is_empty() => {
                    return Err(PalaceError::new(
                        PalaceErrorCode::Forbidden,
                        "publisher identity is required to publish another version",
                    ));
                }
                None => {}
            }
            versions.push(package.clone());
            if let Some(actor_id) = actor_id {
                owners.insert(package.id.clone(), actor_id);
            }
        }

        if let Some(artifact) = artifact {
            self.artifacts.write().await.insert(
                (package.id.clone(), package.version.clone()),
                artifact.clone(),
            );
        }

        self.record_audit_event(&AuditEvent {
            id: Uuid::now_v7(),
            event_type: "package.published".into(),
            actor_id,
            package_id: Some(package.id.clone()),
            details: Some(serde_json::json!({"version": package.version})),
            created_at: Utc::now(),
        })
        .await?;

        Ok(package.clone())
    }

    pub async fn update_package(&self, package: &Package) -> PalaceResult<Package> {
        let _ = package;
        Err(PalaceError::new(
            PalaceErrorCode::ImmutableVersion,
            "published package versions cannot be updated",
        ))
    }
    pub async fn delete_package(&self, id: &str, publisher_id: Uuid) -> PalaceResult<()> {
        let mut map = self.packages.write().await;
        let versions = map
            .get_mut(id)
            .ok_or_else(|| PalaceError::new(PalaceErrorCode::NotFound, "package not found"))?;
        if versions.is_empty() {
            return Err(PalaceError::new(
                PalaceErrorCode::NotFound,
                "package not found",
            ));
        }
        for package in versions {
            package.yanked = true;
        }
        drop(map);
        self.record_audit_event(&AuditEvent {
            id: Uuid::now_v7(),
            event_type: "package.yanked".into(),
            actor_id: Some(publisher_id),
            package_id: Some(id.into()),
            details: None,
            created_at: Utc::now(),
        })
        .await
    }

    pub async fn record_download(&self, id: &str, version: &str) -> PalaceResult<()> {
        self.record_download_with_context(id, version, None)
            .await
            .map(|_| ())
    }

    pub async fn record_download_with_context(
        &self,
        id: &str,
        version: &str,
        dedupe_key: Option<&str>,
    ) -> PalaceResult<bool> {
        if let Some(dedupe_key) = dedupe_key {
            let mut dedup = self.download_dedup.write().await;
            let cutoff = Utc::now() - Duration::hours(1);
            dedup.retain(|_, timestamp| *timestamp >= cutoff);
            let key = (id.to_string(), version.to_string(), dedupe_key.to_string());
            if dedup.contains_key(&key) {
                return Ok(false);
            }

            let mut map = self.packages.write().await;
            let versions = map
                .get_mut(id)
                .ok_or_else(|| PalaceError::new(PalaceErrorCode::NotFound, "package not found"))?;
            if let Some(pkg) = versions.iter_mut().find(|p| p.version == version) {
                pkg.downloads += 1;
                dedup.insert(key, Utc::now());
                return Ok(true);
            }
            return Err(PalaceError::new(
                PalaceErrorCode::NotFound,
                "package version not found",
            ));
        }

        let mut map = self.packages.write().await;
        let versions = map
            .get_mut(id)
            .ok_or_else(|| PalaceError::new(PalaceErrorCode::NotFound, "package not found"))?;
        if let Some(pkg) = versions.iter_mut().find(|p| p.version == version) {
            pkg.downloads += 1;
            return Ok(true);
        }
        Err(PalaceError::new(
            PalaceErrorCode::NotFound,
            "package version not found",
        ))
    }

    pub async fn search(
        &self,
        query: &str,
        pagination: Pagination,
    ) -> PalaceResult<(usize, Vec<Package>)> {
        let filters = PackageFilters {
            q: Some(query.to_string()),
            ..Default::default()
        };
        self.list_packages(filters, pagination).await
    }

    pub async fn featured(&self, limit: usize) -> PalaceResult<Vec<Package>> {
        let mut pkgs = self.packages_filtered(PackageFilters::default()).await;
        pkgs.retain(|p| {
            matches!(
                p.trust.level,
                TrustLevel::Official | TrustLevel::Verified | TrustLevel::Certified
            )
        });
        pkgs.sort_by_key(|b| std::cmp::Reverse(b.downloads));
        Ok(pkgs.into_iter().take(limit).collect())
    }

    pub async fn trending(&self, limit: usize) -> PalaceResult<Vec<Package>> {
        let mut pkgs = self.packages_filtered(PackageFilters::default()).await;
        pkgs.sort_by_key(|b| std::cmp::Reverse(b.downloads));
        Ok(pkgs.into_iter().take(limit).collect())
    }

    pub async fn newest(&self, limit: usize) -> PalaceResult<Vec<Package>> {
        let mut pkgs = self.packages_filtered(PackageFilters::default()).await;
        pkgs.sort_by_key(|b| std::cmp::Reverse(b.created_at));
        Ok(pkgs.into_iter().take(limit).collect())
    }

    pub async fn categories(&self) -> PalaceResult<Vec<String>> {
        let mut cats: Vec<String> = self
            .packages_filtered(PackageFilters::default())
            .await
            .into_iter()
            .flat_map(|p| p.tags)
            .collect();
        cats.sort();
        cats.dedup();
        Ok(cats)
    }

    pub async fn runtimes(&self) -> PalaceResult<Vec<String>> {
        let mut rts: Vec<String> = self
            .packages_filtered(PackageFilters::default())
            .await
            .into_iter()
            .flat_map(|p| p.compatibility.runtimes)
            .collect();
        rts.sort();
        rts.dedup();
        Ok(rts)
    }

    pub async fn add_review(&self, review: &Review) -> PalaceResult<Review> {
        let mut map = self.reviews.write().await;
        let reviews = map.entry(review.package_id.clone()).or_default();
        if reviews
            .iter()
            .any(|existing| existing.reviewer_id == review.reviewer_id)
        {
            return Err(PalaceError::new(
                PalaceErrorCode::Conflict,
                "publisher has already reviewed this package",
            ));
        }
        reviews.push(review.clone());
        Ok(review.clone())
    }

    pub async fn list_reviews(&self, package_id: &str) -> PalaceResult<Vec<Review>> {
        let map = self.reviews.read().await;
        Ok(map
            .get(package_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|review| review.status == ReviewStatus::Published)
            .collect())
    }

    pub async fn moderate_review(
        &self,
        package_id: &str,
        review_id: Uuid,
        status: ReviewStatus,
        moderator_id: Uuid,
        reason: Option<String>,
    ) -> PalaceResult<Review> {
        let mut map = self.reviews.write().await;
        let review = map
            .get_mut(package_id)
            .into_iter()
            .flat_map(|reviews| reviews.iter_mut())
            .find(|review| review.id == review_id)
            .ok_or_else(|| PalaceError::new(PalaceErrorCode::NotFound, "review not found"))?;
        review.status = status;
        review.moderated_by = Some(moderator_id);
        review.moderation_reason = reason;
        review.moderated_at = Some(chrono::Utc::now());
        Ok(review.clone())
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
        let mut reviews = self.reviews.write().await;
        let review = reviews
            .get_mut(package_id)
            .into_iter()
            .flat_map(|items| items.iter_mut())
            .find(|review| review.id == review_id)
            .ok_or_else(|| PalaceError::new(PalaceErrorCode::NotFound, "review not found"))?;
        review.status = status;
        review.moderated_by = Some(moderator_id);
        review.moderation_reason = reason;
        review.moderated_at = Some(chrono::Utc::now());
        let updated = review.clone();
        self.audit.write().await.push(event.clone());
        Ok(updated)
    }

    pub async fn record_trust_transition(&self, transition: &TrustTransition) -> PalaceResult<()> {
        let level = TrustLevel::parse(&transition.to_level).ok_or_else(|| {
            PalaceError::new(
                PalaceErrorCode::ValidationFailed,
                "invalid trust transition level",
            )
        })?;
        let mut packages = self.packages.write().await;
        let versions = packages
            .get_mut(&transition.package_id)
            .ok_or_else(|| PalaceError::new(PalaceErrorCode::NotFound, "package not found"))?;
        if versions.is_empty() {
            return Err(PalaceError::new(
                PalaceErrorCode::NotFound,
                "package not found",
            ));
        }
        for package in versions {
            package.trust.level = level.clone();
        }
        let mut map = self.transitions.write().await;
        map.entry(transition.package_id.clone())
            .or_default()
            .push(transition.clone());
        Ok(())
    }

    pub async fn list_trust_transitions(
        &self,
        package_id: &str,
    ) -> PalaceResult<Vec<TrustTransition>> {
        let map = self.transitions.read().await;
        Ok(map.get(package_id).cloned().unwrap_or_default())
    }

    pub async fn record_audit_event(&self, event: &AuditEvent) -> PalaceResult<()> {
        let mut list = self.audit.write().await;
        list.push(event.clone());
        Ok(())
    }

    pub async fn yank_package(&self, id: &str, version: &str) -> PalaceResult<()> {
        let mut map = self.packages.write().await;
        let versions = map
            .get_mut(id)
            .ok_or_else(|| PalaceError::new(PalaceErrorCode::NotFound, "package not found"))?;
        let pkg = versions
            .iter_mut()
            .find(|p| p.version == version)
            .ok_or_else(|| {
                PalaceError::new(PalaceErrorCode::NotFound, "package version not found")
            })?;
        pkg.yanked = true;
        Ok(())
    }

    pub async fn unyank_package(&self, id: &str, version: &str) -> PalaceResult<()> {
        let mut map = self.packages.write().await;
        let versions = map
            .get_mut(id)
            .ok_or_else(|| PalaceError::new(PalaceErrorCode::NotFound, "package not found"))?;
        let pkg = versions
            .iter_mut()
            .find(|p| p.version == version)
            .ok_or_else(|| {
                PalaceError::new(PalaceErrorCode::NotFound, "package version not found")
            })?;
        pkg.yanked = false;
        Ok(())
    }
}
