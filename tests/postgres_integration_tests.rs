#![cfg(feature = "postgres")]

use chrono::Utc;
use k_o_palace::{
    auth::create_api_token,
    models::{
        AuditEvent, CapabilityInfo, CompatibilityInfo, Package, PackageKind, Publisher, PublisherVerification, Review,
        ReviewStatus, Role, TrustInfo, TrustLevel,
    },
    repository::{postgres::PostgresRepository, PackageRepository},
};
use uuid::Uuid;

fn database_url() -> Option<String> {
    std::env::var("KOP_TEST_DATABASE_URL").ok()
}

fn package(publisher: &str) -> Package {
    let now = Utc::now();
    Package {
        id: format!("{publisher}/postgres-{}", Uuid::new_v4().simple()),
        name: "PostgreSQL integration package".into(),
        version: "1.0.0".into(),
        kind: PackageKind::Gene,
        description: "Database-backed integration test package".into(),
        author: publisher.into(),
        license: "Apache-2.0".into(),
        trust: TrustInfo {
            level: TrustLevel::Community,
            signature: None,
            public_key: None,
            content_hash: None,
            publisher: publisher.into(),
        },
        capabilities: CapabilityInfo::default(),
        downloads: 0,
        success_rate: 0.0,
        compatibility: CompatibilityInfo::default(),
        repository: None,
        artifact_url: None,
        homepage: None,
        tags: vec!["integration-test".into()],
        yanked: false,
        deprecated: None,
        provenance: None,
        created_at: now,
        updated_at: now,
    }
}

#[tokio::test]
async fn postgres_migrates_and_persists_core_registry_records() {
    let Some(url) = database_url() else {
        return;
    };
    let repository = PostgresRepository::new(&url).await.unwrap();
    repository.migrate().await.unwrap();
    let repository = PackageRepository::Postgres(repository);

    let publisher = Publisher {
        id: Uuid::new_v4(),
        name: format!("pgtest{}", Uuid::new_v4().simple()),
        display_name: "PostgreSQL Test Publisher".into(),
        email: Some("postgres-test@example.invalid".into()),
        website: None,
        role: Role::Publisher,
        created_at: Utc::now(),
    };
    let publisher = repository.create_publisher(&publisher).await.unwrap();
    let (plaintext, token) = create_api_token(&repository, publisher.id, "integration")
        .await
        .unwrap();
    assert_eq!(
        repository
            .get_api_token_by_plaintext(&plaintext)
            .await
            .unwrap()
            .id,
        token.id
    );

    let mut package = package(&publisher.name);
    package.trust.level = TrustLevel::Verified;
    package.trust.content_hash = Some("sha256:integration".into());
    package.trust.signature = Some("signature".into());
    package.trust.public_key = Some("public-key".into());
    repository
        .publish_verified_package(&package, None, Some(publisher.id))
        .await
        .unwrap();
    let duplicate = repository.publish_package(&package).await.unwrap_err();
    assert_eq!(
        duplicate.code,
        k_o_palace::error::PalaceErrorCode::ImmutableVersion
    );
    let loaded = repository.get_package(&package.id).await.unwrap();
    assert_eq!(loaded.id, package.id);
    assert_eq!(loaded.trust.level, TrustLevel::Verified);
    assert_eq!(
        loaded.trust.content_hash.as_deref(),
        Some("sha256:integration")
    );
    assert_eq!(
        repository
            .get_package_publisher_id(&package.id)
            .await
            .unwrap(),
        Some(publisher.id)
    );
    assert!(repository
        .record_download_with_context(&package.id, &package.version, Some("integration-key"))
        .await
        .unwrap());
    assert!(!repository
        .record_download_with_context(&package.id, &package.version, Some("integration-key"))
        .await
        .unwrap());
    assert_eq!(
        repository.get_package(&package.id).await.unwrap().downloads,
        1
    );

    repository
        .delete_package(&package.id, publisher.id)
        .await
        .unwrap();
    assert!(repository.get_package(&package.id).await.unwrap().yanked);

    let review = Review {
        id: Uuid::new_v4(),
        package_id: package.id.clone(),
        reviewer_id: publisher.id,
        rating: 5,
        comment: Some("integration test".into()),
        status: ReviewStatus::Published,
        moderated_by: None,
        moderation_reason: None,
        moderated_at: None,
        created_at: Utc::now(),
    };
    let review = repository.add_review(&review).await.unwrap();
    assert_eq!(repository.list_reviews(&package.id).await.unwrap().len(), 1);
    let moderated = repository
        .moderate_review(
            &package.id,
            review.id,
            ReviewStatus::Hidden,
            publisher.id,
            Some("integration moderation".into()),
        )
        .await
        .unwrap();
    assert_eq!(moderated.status, ReviewStatus::Hidden);
    assert!(repository
        .list_reviews(&package.id)
        .await
        .unwrap()
        .is_empty());

    repository
        .record_audit_event(&AuditEvent {
            id: Uuid::new_v4(),
            event_type: "integration.test".into(),
            actor_id: Some(publisher.id),
            package_id: Some(package.id),
            details: None,
            created_at: Utc::now(),
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn postgres_publisher_verification_and_audit_are_atomic() {
    let Some(url) = database_url() else {
        return;
    };
    let repository = PostgresRepository::new(&url).await.unwrap();
    repository.migrate().await.unwrap();
    let repository = PackageRepository::Postgres(repository);

    let target = Publisher {
        id: Uuid::new_v4(),
        name: format!("target{}", Uuid::new_v4().simple()),
        display_name: "Verification Target".into(),
        email: None,
        website: None,
        role: Role::Publisher,
        created_at: Utc::now(),
    };
    let moderator = Publisher {
        id: Uuid::new_v4(),
        name: format!("moderator{}", Uuid::new_v4().simple()),
        display_name: "Verification Moderator".into(),
        email: None,
        website: None,
        role: Role::Moderator,
        created_at: Utc::now(),
    };
    repository.create_publisher(&target).await.unwrap();
    repository.create_publisher(&moderator).await.unwrap();

    let duplicate_audit = AuditEvent {
        id: Uuid::new_v4(),
        event_type: "publisher_verification_updated".into(),
        actor_id: Some(moderator.id),
        package_id: None,
        details: Some(serde_json::json!({
            "publisher_id": target.id,
            "verified": true,
        })),
        created_at: Utc::now(),
    };
    repository.record_audit_event(&duplicate_audit).await.unwrap();

    let verification = PublisherVerification {
        publisher_id: target.id,
        verified: true,
        verified_at: Some(Utc::now()),
        verified_by: Some(moderator.id),
        reason: Some("reviewed".into()),
    };
    let error = repository
        .set_publisher_verification_with_audit(&verification, &duplicate_audit)
        .await
        .unwrap_err();

    assert_eq!(error.code, k_o_palace::error::PalaceErrorCode::Conflict);
    assert_eq!(error.message, "repository operation failed");
    assert!(!repository
        .get_publisher_verification(target.id)
        .await
        .unwrap()
        .verified);
}
