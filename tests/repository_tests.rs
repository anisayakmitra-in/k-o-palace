use chrono::Utc;
use k_o_palace::{
    error::PalaceErrorCode,
    models::{
        AuditEvent, CapabilityInfo, CompatibilityInfo, Package, PackageKind, Publisher, PublisherVerification, Role, TrustInfo,
        TrustLevel,
    },
    repository::{memory::InMemoryRepository, PackageRepository},
};
use uuid::Uuid;

fn publisher(name: &str) -> Publisher {
    Publisher {
        id: Uuid::new_v4(),
        name: name.into(),
        display_name: name.into(),
        email: None,
        website: None,
        role: Role::Publisher,
        created_at: Utc::now(),
    }
}

fn package(id: &str, publisher: &str) -> Package {
    let now = Utc::now();
    Package {
        id: id.into(),
        name: "Repository test package".into(),
        version: "1.0.0".into(),
        kind: PackageKind::Gene,
        description: "Repository boundary test".into(),
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
        tags: Vec::new(),
        yanked: false,
        deprecated: None,
        provenance: None,
        created_at: now,
        updated_at: now,
    }
}

#[tokio::test]
async fn caller_controlled_publisher_metadata_never_assigns_ownership() {
    let repository = PackageRepository::Memory(InMemoryRepository::new());
    let victim = publisher("victim");
    repository.create_publisher(&victim).await.unwrap();

    repository
        .publish_package(&package("metadata-only", &victim.name))
        .await
        .unwrap();

    assert_eq!(
        repository
            .get_package_publisher_id("metadata-only")
            .await
            .unwrap(),
        None
    );
}

#[tokio::test]
async fn anonymous_publish_cannot_claim_a_publisher_namespace() {
    let repository = PackageRepository::Memory(InMemoryRepository::new());
    let victim = publisher("victim");
    repository.create_publisher(&victim).await.unwrap();

    let error = repository
        .publish_package(&package("victim/claimed", &victim.name))
        .await
        .unwrap_err();

    assert_eq!(error.code, PalaceErrorCode::Forbidden);
    assert!(repository.get_package("victim/claimed").await.is_err());
}

#[tokio::test]
async fn publisher_verification_rolls_back_when_its_audit_cannot_be_recorded() {
    let repository = PackageRepository::Memory(InMemoryRepository::new());
    let target = publisher("target");
    let moderator = Publisher {
        role: Role::Moderator,
        ..publisher("moderator")
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

    assert_eq!(error.code, PalaceErrorCode::Conflict);
    assert!(!repository
        .get_publisher_verification(target.id)
        .await
        .unwrap()
        .verified);
}
