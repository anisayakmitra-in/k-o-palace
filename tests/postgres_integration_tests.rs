#![cfg(feature = "postgres")]

use chrono::Utc;
use k_o_palace::{
    auth::create_api_token,
    models::{
        AuditEvent, CapabilityInfo, CompatibilityInfo, Package, PackageKind, Publisher, Review,
        Role, TrustInfo, TrustLevel,
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

    let package = package(&publisher.name);
    repository.publish_package(&package).await.unwrap();
    assert_eq!(
        repository.get_package(&package.id).await.unwrap().id,
        package.id
    );

    let review = Review {
        id: Uuid::new_v4(),
        package_id: package.id.clone(),
        reviewer_id: publisher.id,
        rating: 5,
        comment: Some("integration test".into()),
        created_at: Utc::now(),
    };
    repository.add_review(&review).await.unwrap();
    assert_eq!(repository.list_reviews(&package.id).await.unwrap().len(), 1);

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
