//! Trust transition and signature tests.

use chrono::Utc;
use k_o_palace::{
    auth::AuthContext,
    models::{Publisher, PublisherVerification, Role, TrustInfo, TrustLevel},
    trust::{can_transition, transition_trust, transition_trust_with_policy, verify_signature},
};
use uuid::Uuid;

#[test]
fn experimental_to_community_allowed() {
    assert!(can_transition(
        TrustLevel::Experimental,
        TrustLevel::Community
    ));
}

#[test]
fn experimental_to_certified_allowed() {
    assert!(can_transition(
        TrustLevel::Experimental,
        TrustLevel::Certified
    ));
}

#[test]
fn certified_cannot_advance() {
    assert!(!can_transition(TrustLevel::Certified, TrustLevel::Official));
}

#[test]
fn same_level_allowed() {
    assert!(can_transition(TrustLevel::Verified, TrustLevel::Verified));
}

#[test]
fn forged_signature_fails() {
    let trust = TrustInfo {
        level: TrustLevel::Verified,
        signature: Some("aGVsbG8=".into()),
        public_key: Some("aGVsbG8=".into()),
        content_hash: None,
        publisher: "attacker".into(),
    };
    let err = verify_signature(&trust, b"content").unwrap_err();
    assert_eq!(
        err.code,
        k_o_palace::error::PalaceErrorCode::SignatureInvalid
    );
}

#[tokio::test]
async fn moderator_can_transition_trust() {
    use k_o_palace::models::Package;
    use k_o_palace::repository::memory::InMemoryRepository;
    use k_o_palace::repository::PackageRepository;

    let repo = PackageRepository::Memory(InMemoryRepository::new());
    let publisher = Publisher {
        id: Uuid::new_v4(),
        name: "mod".into(),
        display_name: "Moderator".into(),
        email: None,
        website: None,
        role: Role::Moderator,
        created_at: Utc::now(),
    };
    repo.create_publisher(&publisher).await.unwrap();

    let pkg = Package {
        id: "test.gene".into(),
        name: "Test".into(),
        version: "1.0.0".into(),
        kind: k_o_palace::models::PackageKind::Gene,
        description: "test".into(),
        author: "test".into(),
        license: "MIT".into(),
        trust: TrustInfo {
            level: TrustLevel::Experimental,
            signature: None,
            public_key: None,
            content_hash: None,
            publisher: "test".into(),
        },
        capabilities: Default::default(),
        downloads: 0,
        success_rate: 0.0,
        compatibility: Default::default(),
        repository: None,
        artifact_url: None,
        homepage: None,
        tags: vec![],
        yanked: false,
        provenance: None,
        deprecated: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    repo.publish_package(&pkg).await.unwrap();

    let ctx = AuthContext {
        publisher: publisher.clone(),
        token_id: Uuid::new_v4(),
        scopes: vec!["moderation:write".into()],
    };
    let transition = transition_trust(
        &repo,
        &ctx,
        "test.gene",
        TrustLevel::Verified,
        Some("reviewed".into()),
    )
    .await
    .unwrap();
    assert_eq!(transition.to_level, "verified");
    assert_eq!(
        repo.get_package("test.gene").await.unwrap().trust.level,
        TrustLevel::Verified
    );
}

#[tokio::test]
async fn trust_transition_updates_all_versions_in_memory() {
    use k_o_palace::models::Package;
    use k_o_palace::repository::memory::InMemoryRepository;
    use k_o_palace::repository::PackageRepository;

    let repo = PackageRepository::Memory(InMemoryRepository::new());
    let publisher = Publisher {
        id: Uuid::new_v4(),
        name: "mod".into(),
        display_name: "Moderator".into(),
        email: None,
        website: None,
        role: Role::Moderator,
        created_at: Utc::now(),
    };
    repo.create_publisher(&publisher).await.unwrap();
    repo.set_publisher_verification(&PublisherVerification {
        publisher_id: publisher.id,
        verified: true,
        verified_at: Some(Utc::now()),
        verified_by: Some(publisher.id),
        reason: Some("test publisher".into()),
    })
    .await
    .unwrap();

    let mut first = Package {
        id: "versions.gene".into(),
        name: "Versions".into(),
        version: "1.0.0".into(),
        kind: k_o_palace::models::PackageKind::Gene,
        description: "test".into(),
        author: "test".into(),
        license: "MIT".into(),
        trust: TrustInfo {
            level: TrustLevel::Experimental,
            signature: None,
            public_key: None,
            content_hash: None,
            publisher: "mod".into(),
        },
        capabilities: Default::default(),
        downloads: 0,
        success_rate: 0.0,
        compatibility: Default::default(),
        repository: None,
        artifact_url: None,
        homepage: None,
        tags: vec![],
        yanked: false,
        provenance: None,
        deprecated: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    repo.publish_package(&first).await.unwrap();
    first.version = "2.0.0".into();
    repo.publish_package(&first).await.unwrap();

    let ctx = AuthContext {
        publisher,
        token_id: Uuid::new_v4(),
        scopes: vec!["moderation:write".into()],
    };
    transition_trust(
        &repo,
        &ctx,
        "versions.gene",
        TrustLevel::Verified,
        Some("reviewed".into()),
    )
    .await
    .unwrap();

    assert_eq!(
        repo.get_package_version("versions.gene", "1.0.0")
            .await
            .unwrap()
            .trust
            .level,
        TrustLevel::Verified
    );
    assert_eq!(
        repo.get_package_version("versions.gene", "2.0.0")
            .await
            .unwrap()
            .trust
            .level,
        TrustLevel::Verified
    );
}
#[tokio::test]
async fn server_assigned_trust_requires_verified_publisher() {
    use k_o_palace::models::{CapabilityInfo, CompatibilityInfo, Package, PackageKind};

    let repo = k_o_palace::repository::PackageRepository::Memory(
        k_o_palace::repository::memory::InMemoryRepository::new(),
    );
    let owner = Publisher {
        id: Uuid::new_v4(),
        name: "owner".into(),
        display_name: "Owner".into(),
        email: None,
        website: None,
        role: Role::Publisher,
        created_at: Utc::now(),
    };
    let moderator = Publisher {
        id: Uuid::new_v4(),
        name: "moderator".into(),
        display_name: "Moderator".into(),
        email: None,
        website: None,
        role: Role::Moderator,
        created_at: Utc::now(),
    };
    repo.create_publisher(&owner).await.unwrap();
    repo.create_publisher(&moderator).await.unwrap();

    let package = Package {
        id: "owned.gene".into(),
        name: "Owned Gene".into(),
        version: "1.0.0".into(),
        kind: PackageKind::Gene,
        description: "owned".into(),
        author: owner.name.clone(),
        license: "MIT".into(),
        trust: TrustInfo {
            level: TrustLevel::Experimental,
            signature: None,
            public_key: None,
            content_hash: None,
            publisher: owner.name.clone(),
        },
        capabilities: CapabilityInfo::default(),
        downloads: 0,
        success_rate: 0.0,
        compatibility: CompatibilityInfo::default(),
        repository: None,
        artifact_url: None,
        homepage: None,
        tags: vec![],
        yanked: false,
        deprecated: None,
        provenance: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    repo.publish_package(&package).await.unwrap();

    let context = AuthContext {
        publisher: moderator.clone(),
        token_id: Uuid::new_v4(),
        scopes: vec!["moderation:write".into()],
    };
    let rejected = transition_trust(
        &repo,
        &context,
        "owned.gene",
        TrustLevel::Verified,
        Some("not yet verified".into()),
    )
    .await
    .unwrap_err();
    assert_eq!(
        rejected.code,
        k_o_palace::error::PalaceErrorCode::TrustTransitionDenied
    );

    repo.set_publisher_verification(&PublisherVerification {
        publisher_id: owner.id,
        verified: true,
        verified_at: Some(Utc::now()),
        verified_by: Some(moderator.id),
        reason: Some("reviewed".into()),
    })
    .await
    .unwrap();

    let signature_rejected = transition_trust_with_policy(
        &repo,
        &context,
        "owned.gene",
        TrustLevel::Verified,
        Some("signature required".into()),
        true,
    )
    .await
    .unwrap_err();
    assert_eq!(
        signature_rejected.code,
        k_o_palace::error::PalaceErrorCode::TrustTransitionDenied
    );

    let transition = transition_trust(
        &repo,
        &context,
        "owned.gene",
        TrustLevel::Verified,
        Some("approved".into()),
    )
    .await
    .unwrap();
    assert_eq!(transition.to_level, "verified");

    let mut orphan = package.clone();
    orphan.id = "orphaned.gene".into();
    orphan.author = "missing-owner".into();
    orphan.trust.publisher = "missing-owner".into();
    repo.publish_package(&orphan).await.unwrap();

    let orphan_rejected = transition_trust_with_policy(
        &repo,
        &context,
        "orphaned.gene",
        TrustLevel::Verified,
        Some("publisher required".into()),
        true,
    )
    .await
    .unwrap_err();
    assert_eq!(
        orphan_rejected.code,
        k_o_palace::error::PalaceErrorCode::TrustTransitionDenied
    );
}
