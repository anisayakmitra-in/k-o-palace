//! PostgreSQL repository implementation (feature-gated).

#![cfg(feature = "postgres")]

use crate::error::{PalaceError, PalaceErrorCode, PalaceResult};
use crate::models::*;
use crate::pagination::Pagination;
use crate::repository::PackageFilters;
use chrono::Utc;
use sqlx::postgres::PgPool;
use uuid::Uuid;

/// Helper struct for SQLx row mapping.
#[derive(sqlx::FromRow)]
struct PackageRow {
    id: String,
    name: String,
    version: String,
    kind: String,
    description: String,
    author: String,
    license: String,
    repository: Option<String>,
    artifact_url: Option<String>,
    homepage: Option<String>,
    tags: serde_json::Value,
    capabilities: serde_json::Value,
    compatibility: serde_json::Value,
    downloads: i64,
    success_rate: f32,
    yanked: bool,
    deprecated: Option<String>,
    provenance: Option<serde_json::Value>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

impl From<PackageRow> for Package {
    fn from(r: PackageRow) -> Self {
        Package {
            id: r.id,
            name: r.name,
            version: r.version,
            kind: PackageKind::parse(&r.kind).unwrap_or(PackageKind::Gene),
            description: r.description,
            author: r.author.clone(),
            license: r.license,
            trust: TrustInfo {
                level: TrustLevel::Community,
                signature: None,
                public_key: None,
                content_hash: None,
                publisher: r.author,
            },
            repository: r.repository,
            artifact_url: r.artifact_url,
            homepage: r.homepage,
            tags: serde_json::from_value(r.tags).unwrap_or_default(),
            capabilities: serde_json::from_value(r.capabilities).unwrap_or_default(),
            compatibility: serde_json::from_value(r.compatibility).unwrap_or_default(),
            downloads: r.downloads as u64,
            success_rate: r.success_rate as f64,
            yanked: r.yanked,
            deprecated: r.deprecated,
            provenance: r.provenance.and_then(|v| serde_json::from_value(v).ok()),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// PostgreSQL-backed repository.
#[derive(Debug, Clone)]
pub struct PostgresRepository {
    pool: PgPool,
}

impl PostgresRepository {
    pub async fn new(url: &str) -> PalaceResult<Self> {
        let pool = PgPool::connect(url)
            .await
            .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))?;
        Ok(Self { pool })
    }

    pub async fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn migrate(&self) -> PalaceResult<()> {
        sqlx::migrate!()
            .run(&self.pool)
            .await
            .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))
    }

    pub async fn is_healthy(&self) -> bool {
        sqlx::query("SELECT 1").execute(&self.pool).await.is_ok()
    }

    pub async fn create_publisher(&self, publisher: &Publisher) -> PalaceResult<Publisher> {
        let row = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                String,
                Option<String>,
                Option<String>,
                String,
                chrono::DateTime<Utc>,
            ),
        >(
            "INSERT INTO publishers (id, name, display_name, email, website, role, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (name) DO NOTHING
             RETURNING id, name, display_name, email, website, role, created_at",
        )
        .bind(publisher.id)
        .bind(&publisher.name)
        .bind(&publisher.display_name)
        .bind(&publisher.email)
        .bind(&publisher.website)
        .bind(publisher.role.as_str())
        .bind(publisher.created_at)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))?;

        if let Some(r) = row {
            Ok(Publisher {
                id: r.0,
                name: r.1,
                display_name: r.2,
                email: r.3,
                website: r.4,
                role: Role::parse(&r.5).unwrap_or(Role::Publisher),
                created_at: r.6,
            })
        } else {
            Err(PalaceError::new(
                PalaceErrorCode::Conflict,
                "publisher name already exists",
            ))
        }
    }

    pub async fn get_publisher_by_id(&self, id: Uuid) -> PalaceResult<Publisher> {
        let row = sqlx::query_as::<_, (Uuid, String, String, Option<String>, Option<String>, String, chrono::DateTime<Utc>)>(
            "SELECT id, name, display_name, email, website, role, created_at FROM publishers WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))?
        .ok_or_else(|| PalaceError::new(PalaceErrorCode::NotFound, "publisher not found"))?;

        Ok(Publisher {
            id: row.0,
            name: row.1,
            display_name: row.2,
            email: row.3,
            website: row.4,
            role: Role::parse(&row.5).unwrap_or(Role::Publisher),
            created_at: row.6,
        })
    }

    pub async fn get_publisher_by_name(&self, name: &str) -> PalaceResult<Publisher> {
        let row = sqlx::query_as::<_, (Uuid, String, String, Option<String>, Option<String>, String, chrono::DateTime<Utc>)>(
            "SELECT id, name, display_name, email, website, role, created_at FROM publishers WHERE name = $1"
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))?
        .ok_or_else(|| PalaceError::new(PalaceErrorCode::NotFound, "publisher not found"))?;

        Ok(Publisher {
            id: row.0,
            name: row.1,
            display_name: row.2,
            email: row.3,
            website: row.4,
            role: Role::parse(&row.5).unwrap_or(Role::Publisher),
            created_at: row.6,
        })
    }

    pub async fn update_publisher_role(&self, id: Uuid, role: Role) -> PalaceResult<Publisher> {
        let row = sqlx::query_as::<_, (Uuid, String, String, Option<String>, Option<String>, String, chrono::DateTime<Utc>)>(
            "UPDATE publishers SET role = $2 WHERE id = $1 RETURNING id, name, display_name, email, website, role, created_at"
        )
        .bind(id)
        .bind(role.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))?
        .ok_or_else(|| PalaceError::new(PalaceErrorCode::NotFound, "publisher not found"))?;

        Ok(Publisher {
            id: row.0,
            name: row.1,
            display_name: row.2,
            email: row.3,
            website: row.4,
            role: Role::parse(&row.5).unwrap_or(Role::Publisher),
            created_at: row.6,
        })
    }

    pub async fn create_api_token(&self, token: &ApiToken) -> PalaceResult<()> {
        sqlx::query(
            "INSERT INTO api_tokens (id, publisher_id, token_hash, name, created_at, revoked_at, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)"
        )
        .bind(token.id)
        .bind(token.publisher_id)
        .bind(&token.token_hash)
        .bind(&token.name)
        .bind(token.created_at)
        .bind(token.revoked_at)
        .bind(token.expires_at)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))
    }

    pub async fn get_api_token_by_plaintext(&self, plaintext: &str) -> PalaceResult<ApiToken> {
        let rows = sqlx::query_as::<_, (Uuid, Uuid, String, String, chrono::DateTime<Utc>, Option<chrono::DateTime<Utc>>, Option<chrono::DateTime<Utc>>)>(
            "SELECT id, publisher_id, token_hash, name, created_at, revoked_at, expires_at FROM api_tokens WHERE revoked_at IS NULL"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))?;

        for row in rows {
            if bcrypt::verify(plaintext.as_bytes(), &row.2).unwrap_or(false) {
                return Ok(ApiToken {
                    id: row.0,
                    publisher_id: row.1,
                    token_hash: row.2,
                    name: row.3,
                    created_at: row.4,
                    revoked_at: row.5,
                    expires_at: row.6,
                });
            }
        }
        Err(PalaceError::new(
            PalaceErrorCode::Unauthorized,
            "invalid or revoked token",
        ))
    }

    pub async fn revoke_api_token(&self, id: Uuid) -> PalaceResult<()> {
        sqlx::query("UPDATE api_tokens SET revoked_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))
    }

    pub async fn list_api_tokens(&self, publisher_id: Uuid) -> PalaceResult<Vec<ApiToken>> {
        let rows = sqlx::query_as::<_, (Uuid, Uuid, String, String, chrono::DateTime<Utc>, Option<chrono::DateTime<Utc>>, Option<chrono::DateTime<Utc>>)>(
            "SELECT id, publisher_id, token_hash, name, created_at, revoked_at, expires_at FROM api_tokens WHERE publisher_id = $1 ORDER BY created_at DESC"
        )
        .bind(publisher_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| ApiToken {
                id: r.0,
                publisher_id: r.1,
                token_hash: r.2,
                name: r.3,
                created_at: r.4,
                revoked_at: r.5,
                expires_at: r.6,
            })
            .collect())
    }

    pub async fn list_packages(
        &self,
        _filters: PackageFilters,
        pagination: Pagination,
    ) -> PalaceResult<(usize, Vec<Package>)> {
        let rows: Vec<PackageRow> = sqlx::query_as::<_, PackageRow>(
            "SELECT id, name, version, kind, description, author, license, repository, artifact_url, homepage, tags, capabilities, compatibility, downloads, success_rate, yanked, deprecated, provenance, created_at, updated_at FROM packages ORDER BY created_at DESC LIMIT $1 OFFSET $2"
        )
        .bind(pagination.limit as i64)
        .bind(pagination.offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))?;

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM packages")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))?;

        Ok((
            total as usize,
            rows.into_iter().map(Package::from).collect(),
        ))
    }

    pub async fn get_package(&self, id: &str) -> PalaceResult<Package> {
        let row = sqlx::query_as::<_, PackageRow>(
            "SELECT id, name, version, kind, description, author, license, repository, artifact_url, homepage, tags, capabilities, compatibility, downloads, success_rate, yanked, deprecated, provenance, created_at, updated_at FROM packages WHERE id = $1 ORDER BY created_at DESC LIMIT 1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))?
        .ok_or_else(|| PalaceError::new(PalaceErrorCode::NotFound, "package not found"))?;

        Ok(row.into())
    }

    pub async fn get_package_version(&self, id: &str, version: &str) -> PalaceResult<Package> {
        let row = sqlx::query_as::<_, PackageRow>(
            "SELECT id, name, version, kind, description, author, license, repository, artifact_url, homepage, tags, capabilities, compatibility, downloads, success_rate, yanked, deprecated, provenance, created_at, updated_at FROM packages WHERE id = $1 AND version = $2"
        )
        .bind(id)
        .bind(version)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))?
        .ok_or_else(|| PalaceError::new(PalaceErrorCode::NotFound, "package version not found"))?;

        Ok(row.into())
    }

    pub async fn list_versions(&self, id: &str) -> PalaceResult<Vec<VersionInfo>> {
        let rows = sqlx::query_as::<_, (String, chrono::DateTime<Utc>, Option<String>, Option<String>)>(
            "SELECT version, created_at, artifact_url, NULL as content_hash FROM packages WHERE id = $1 ORDER BY created_at DESC"
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| VersionInfo {
                version: r.0,
                created_at: r.1,
                artifact_url: r.2,
                content_hash: r.3,
            })
            .collect())
    }

    pub async fn publish_package(&self, package: &Package) -> PalaceResult<Package> {
        let tags_json = serde_json::to_value(&package.tags).unwrap_or_default();
        let caps_json = serde_json::to_value(&package.capabilities).unwrap_or_default();
        let compat_json = serde_json::to_value(&package.compatibility).unwrap_or_default();

        sqlx::query(
            "INSERT INTO packages (id, name, version, kind, description, author, license, repository, artifact_url, homepage, tags, capabilities, compatibility, downloads, success_rate, yanked, deprecated, provenance, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)
             ON CONFLICT (id, version) DO UPDATE SET
                name = EXCLUDED.name, description = EXCLUDED.description, updated_at = NOW()"
        )
        .bind(&package.id)
        .bind(&package.name)
        .bind(&package.version)
        .bind(package.kind.as_str())
        .bind(&package.description)
        .bind(&package.author)
        .bind(&package.license)
        .bind(&package.repository)
        .bind(&package.artifact_url)
        .bind(&package.homepage)
        .bind(&tags_json)
        .bind(&caps_json)
        .bind(&compat_json)
        .bind(package.downloads as i64)
        .bind(package.success_rate)
        .bind(package.yanked)
        .bind(&package.deprecated)
        .bind(serde_json::to_value(&package.provenance).unwrap_or(serde_json::Value::Null))
        .bind(package.created_at)
        .bind(package.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))?;

        Ok(package.clone())
    }

    pub async fn update_package(&self, package: &Package) -> PalaceResult<Package> {
        self.publish_package(package).await
    }

    pub async fn delete_package(&self, id: &str, _publisher_id: Uuid) -> PalaceResult<()> {
        let result = sqlx::query("DELETE FROM packages WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))?;
        if result.rows_affected() == 0 {
            return Err(PalaceError::new(
                PalaceErrorCode::NotFound,
                "package not found",
            ));
        }
        Ok(())
    }

    pub async fn record_download(&self, id: &str, version: &str) -> PalaceResult<()> {
        sqlx::query("UPDATE packages SET downloads = downloads + 1 WHERE id = $1 AND version = $2")
            .bind(id)
            .bind(version)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))
    }

    pub async fn search(
        &self,
        query: &str,
        pagination: Pagination,
    ) -> PalaceResult<(usize, Vec<Package>)> {
        let pattern = format!("%{query}%");
        let rows: Vec<PackageRow> = sqlx::query_as::<_, PackageRow>(
            "SELECT id, name, version, kind, description, author, license, repository, artifact_url, homepage, tags, capabilities, compatibility, downloads, success_rate, yanked, deprecated, provenance, created_at, updated_at FROM packages WHERE name ILIKE $1 OR description ILIKE $1 ORDER BY downloads DESC LIMIT $2 OFFSET $3"
        )
        .bind(&pattern)
        .bind(pagination.limit as i64)
        .bind(pagination.offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))?;

        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM packages WHERE name ILIKE $1 OR description ILIKE $1",
        )
        .bind(&pattern)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))?;

        Ok((
            total as usize,
            rows.into_iter().map(Package::from).collect(),
        ))
    }

    pub async fn featured(&self, limit: usize) -> PalaceResult<Vec<Package>> {
        let rows: Vec<PackageRow> = sqlx::query_as::<_, PackageRow>(
            "SELECT id, name, version, kind, description, author, license, repository, artifact_url, homepage, tags, capabilities, compatibility, downloads, success_rate, yanked, deprecated, provenance, created_at, updated_at FROM packages ORDER BY success_rate DESC, downloads DESC LIMIT $1"
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))?;

        Ok(rows.into_iter().map(Package::from).collect())
    }

    pub async fn trending(&self, limit: usize) -> PalaceResult<Vec<Package>> {
        let rows: Vec<PackageRow> = sqlx::query_as::<_, PackageRow>(
            "SELECT id, name, version, kind, description, author, license, repository, artifact_url, homepage, tags, capabilities, compatibility, downloads, success_rate, yanked, deprecated, provenance, created_at, updated_at FROM packages ORDER BY downloads DESC LIMIT $1"
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))?;

        Ok(rows.into_iter().map(Package::from).collect())
    }

    pub async fn newest(&self, limit: usize) -> PalaceResult<Vec<Package>> {
        let rows: Vec<PackageRow> = sqlx::query_as::<_, PackageRow>(
            "SELECT id, name, version, kind, description, author, license, repository, artifact_url, homepage, tags, capabilities, compatibility, downloads, success_rate, yanked, deprecated, provenance, created_at, updated_at FROM packages ORDER BY created_at DESC LIMIT $1"
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))?;

        Ok(rows.into_iter().map(Package::from).collect())
    }

    pub async fn categories(&self) -> PalaceResult<Vec<String>> {
        let rows: Vec<(serde_json::Value,)> = sqlx::query_as("SELECT tags FROM packages")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))?;

        let mut cats = std::collections::BTreeSet::new();
        for (tags,) in rows {
            if let Ok(arr) = serde_json::from_value::<Vec<String>>(tags) {
                for tag in arr {
                    cats.insert(tag);
                }
            }
        }
        Ok(cats.into_iter().collect())
    }

    pub async fn runtimes(&self) -> PalaceResult<Vec<String>> {
        let rows: Vec<(serde_json::Value,)> = sqlx::query_as("SELECT compatibility FROM packages")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))?;

        let mut runtimes = std::collections::BTreeSet::new();
        for (compat,) in rows {
            if let Ok(ci) = serde_json::from_value::<CompatibilityInfo>(compat) {
                for rt in ci.runtimes.iter() {
                    runtimes.insert(rt.clone());
                }
            }
        }
        Ok(runtimes.into_iter().collect())
    }

    pub async fn add_review(&self, review: &Review) -> PalaceResult<Review> {
        let id = uuid::Uuid::now_v7();
        sqlx::query(
            "INSERT INTO reviews (id, package_id, reviewer_id, rating, comment, created_at) VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(id)
        .bind(&review.package_id)
        .bind(review.reviewer_id)
        .bind(review.rating)
        .bind(&review.comment)
        .bind(review.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))?;

        Ok(Review {
            id,
            ..review.clone()
        })
    }

    pub async fn list_reviews(&self, package_id: &str) -> PalaceResult<Vec<Review>> {
        let rows = sqlx::query_as::<_, (Uuid, String, Uuid, i16, Option<String>, chrono::DateTime<Utc>)>(
            "SELECT id, package_id, reviewer_id, rating, comment, created_at FROM reviews WHERE package_id = $1 ORDER BY created_at DESC"
        )
        .bind(package_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| Review {
                id: r.0,
                package_id: r.1,
                reviewer_id: r.2,
                rating: r.3,
                comment: r.4,
                created_at: r.5,
            })
            .collect())
    }

    pub async fn record_trust_transition(&self, transition: &TrustTransition) -> PalaceResult<()> {
        let id = uuid::Uuid::now_v7();
        sqlx::query(
            "INSERT INTO trust_transitions (id, package_id, from_level, to_level, approved_by, reason, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)"
        )
        .bind(id)
        .bind(&transition.package_id)
        .bind(&transition.from_level)
        .bind(&transition.to_level)
        .bind(transition.approved_by)
        .bind(&transition.reason)
        .bind(transition.created_at)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))
    }

    pub async fn list_trust_transitions(
        &self,
        package_id: &str,
    ) -> PalaceResult<Vec<TrustTransition>> {
        let rows = sqlx::query_as::<_, (Uuid, String, String, String, Option<Uuid>, Option<String>, chrono::DateTime<Utc>)>(
            "SELECT id, package_id, from_level, to_level, approved_by, reason, created_at FROM trust_transitions WHERE package_id = $1 ORDER BY created_at DESC"
        )
        .bind(package_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| TrustTransition {
                id: r.0,
                package_id: r.1,
                from_level: r.2,
                to_level: r.3,
                approved_by: r.4.unwrap_or_default(),
                reason: r.5,
                created_at: r.6,
            })
            .collect())
    }

    pub async fn record_audit_event(&self, event: &AuditEvent) -> PalaceResult<()> {
        let id = uuid::Uuid::now_v7();
        sqlx::query(
            "INSERT INTO audit_events (id, event_type, actor_id, package_id, details, created_at) VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(id)
        .bind(&event.event_type)
        .bind(event.actor_id)
        .bind(&event.package_id)
        .bind(&event.details)
        .bind(event.created_at)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))
    }

    pub async fn yank_package(&self, id: &str, version: &str) -> PalaceResult<()> {
        let result =
            sqlx::query("UPDATE packages SET yanked = true WHERE id = $1 AND version = $2")
                .bind(id)
                .bind(version)
                .execute(&self.pool)
                .await
                .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))?;
        if result.rows_affected() == 0 {
            return Err(PalaceError::new(
                PalaceErrorCode::NotFound,
                "package version not found",
            ));
        }
        Ok(())
    }

    pub async fn unyank_package(&self, id: &str, version: &str) -> PalaceResult<()> {
        let result =
            sqlx::query("UPDATE packages SET yanked = false WHERE id = $1 AND version = $2")
                .bind(id)
                .bind(version)
                .execute(&self.pool)
                .await
                .map_err(|e| PalaceError::new(PalaceErrorCode::ServerError, e.to_string()))?;
        if result.rows_affected() == 0 {
            return Err(PalaceError::new(
                PalaceErrorCode::NotFound,
                "package version not found",
            ));
        }
        Ok(())
    }
}
