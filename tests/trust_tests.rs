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
        TrustLevel::Community,
        Some("reviewed".into()),
    )
    .await
    .unwrap();
    assert_eq!(transition.to_level, "community");
    assert_eq!(
        repo.get_package("test.gene").await.unwrap().trust.level,
        TrustLevel::Community
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
    let publisher_id = publisher.id;

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
    repo.publish_verified_package(&first, None, Some(publisher_id))
        .await
        .unwrap();
    first.version = "2.0.0".into();
    repo.publish_verified_package(&first, None, Some(publisher_id))
        .await
        .unwrap();

    let ctx = AuthContext {
        publisher,
        token_id: Uuid::new_v4(),
        scopes: vec!["moderation:write".into()],
    };
    transition_trust(
        &repo,
        &ctx,
        "versions.gene",
        TrustLevel::Community,
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
        TrustLevel::Community
    );
    assert_eq!(
        repo.get_package_version("versions.gene", "2.0.0")
            .await
            .unwrap()
            .trust
            .level,
        TrustLevel::Community
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

    let evidence_rejected = transition_trust(
        &repo,
        &context,
        "owned.gene",
        TrustLevel::Verified,
        Some("artifact evidence required".into()),
    )
    .await
    .unwrap_err();
    assert_eq!(
        evidence_rejected.code,
        k_o_palace::error::PalaceErrorCode::TrustTransitionDenied
    );
    assert!(evidence_rejected
        .message
        .contains("server-recorded artifact"));

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

#[tokio::test]
async fn server_assigned_trust_rejects_client_supplied_signature_metadata() {
    use k_o_palace::models::{CapabilityInfo, CompatibilityInfo, Package, PackageKind};
    use k_o_palace::repository::{memory::InMemoryRepository, PackageRepository};

    let repo = PackageRepository::Memory(InMemoryRepository::new());
    let owner = Publisher {
        id: Uuid::new_v4(),
        name: "signed-owner".into(),
        display_name: "Signed Owner".into(),
        email: None,
        website: None,
        role: Role::Publisher,
        created_at: Utc::now(),
    };
    let moderator = Publisher {
        id: Uuid::new_v4(),
        name: "signed-moderator".into(),
        display_name: "Signed Moderator".into(),
        email: None,
        website: None,
        role: Role::Moderator,
        created_at: Utc::now(),
    };
    repo.create_publisher(&owner).await.unwrap();
    repo.create_publisher(&moderator).await.unwrap();
    repo.set_publisher_verification(&PublisherVerification {
        publisher_id: owner.id,
        verified: true,
        verified_at: Some(Utc::now()),
        verified_by: Some(moderator.id),
        reason: Some("publisher identity verified".into()),
    })
    .await
    .unwrap();

    let package = Package {
        id: "metadata-only.gene".into(),
        name: "Metadata Only".into(),
        version: "1.0.0".into(),
        kind: PackageKind::Gene,
        description: "client supplied trust metadata".into(),
        author: owner.name.clone(),
        license: "MIT".into(),
        trust: TrustInfo {
            level: TrustLevel::Experimental,
            signature: Some("client-supplied-signature".into()),
            public_key: Some("client-supplied-public-key".into()),
            content_hash: Some("sha256:client-supplied-hash".into()),
            publisher: owner.name.clone(),
        },
        capabilities: CapabilityInfo::default(),
        downloads: 0,
        success_rate: 0.0,
        compatibility: CompatibilityInfo::default(),
        repository: None,
        artifact_url: Some("https://example.com/metadata-only.tar.gz".into()),
        homepage: None,
        tags: vec![],
        yanked: false,
        deprecated: None,
        provenance: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    repo.publish_verified_package(&package, None, Some(owner.id))
        .await
        .unwrap();

    let context = AuthContext {
        publisher: moderator,
        token_id: Uuid::new_v4(),
        scopes: vec!["moderation:write".into()],
    };
    let rejected = transition_trust_with_policy(
        &repo,
        &context,
        &package.id,
        TrustLevel::Verified,
        Some("metadata is not evidence".into()),
        true,
    )
    .await
    .unwrap_err();

    assert_eq!(
        rejected.code,
        k_o_palace::error::PalaceErrorCode::TrustTransitionDenied
    );
    assert!(rejected.message.contains("server-recorded artifact"));
}
#[tokio::test]
async fn server_assigned_trust_requires_durable_evidence_for_every_version() {
    use k_o_palace::models::{CapabilityInfo, CompatibilityInfo, Package, PackageKind};
    use k_o_palace::repository::{memory::InMemoryRepository, PackageRepository, VerifiedArtifact};

    let repo = PackageRepository::Memory(InMemoryRepository::new());
    let owner = Publisher {
        id: Uuid::new_v4(),
        name: "artifact-owner".into(),
        display_name: "Artifact Owner".into(),
        email: None,
        website: None,
        role: Role::Publisher,
        created_at: Utc::now(),
    };
    let moderator = Publisher {
        id: Uuid::new_v4(),
        name: "artifact-moderator".into(),
        display_name: "Artifact Moderator".into(),
        email: None,
        website: None,
        role: Role::Moderator,
        created_at: Utc::now(),
    };
    repo.create_publisher(&owner).await.unwrap();
    repo.create_publisher(&moderator).await.unwrap();
    repo.set_publisher_verification(&PublisherVerification {
        publisher_id: owner.id,
        verified: true,
        verified_at: Some(Utc::now()),
        verified_by: Some(moderator.id),
        reason: Some("publisher identity verified".into()),
    })
    .await
    .unwrap();

    let mut package = Package {
        id: "artifact-evidence.gene".into(),
        name: "Artifact Evidence".into(),
        version: "1.0.0".into(),
        kind: PackageKind::Gene,
        description: "durably verified artifact".into(),
        author: owner.name.clone(),
        license: "MIT".into(),
        trust: TrustInfo {
            level: TrustLevel::Experimental,
            signature: None,
            public_key: None,
            content_hash: Some("sha256:verified".into()),
            publisher: owner.name.clone(),
        },
        capabilities: CapabilityInfo::default(),
        downloads: 0,
        success_rate: 0.0,
        compatibility: CompatibilityInfo::default(),
        repository: None,
        artifact_url: Some("https://example.com/v1.tar.gz".into()),
        homepage: None,
        tags: vec![],
        yanked: false,
        deprecated: None,
        provenance: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let artifact = VerifiedArtifact {
        url: package.artifact_url.clone().unwrap(),
        content_type: Some("application/gzip".into()),
        size_bytes: 128,
        content_hash: "sha256:verified".into(),
        signature: None,
        public_key: None,
    };
    repo.publish_verified_package(&package, Some(&artifact), Some(owner.id))
        .await
        .unwrap();

    package.version = "2.0.0".into();
    package.artifact_url = None;
    package.trust.content_hash = None;
    package.created_at += chrono::Duration::seconds(1);
    repo.publish_verified_package(&package, None, Some(owner.id))
        .await
        .unwrap();

    let context = AuthContext {
        publisher: moderator,
        token_id: Uuid::new_v4(),
        scopes: vec!["moderation:write".into()],
    };
    let rejected = transition_trust(
        &repo,
        &context,
        &package.id,
        TrustLevel::Verified,
        Some("old evidence is insufficient".into()),
    )
    .await
    .unwrap_err();
    assert_eq!(
        rejected.code,
        k_o_palace::error::PalaceErrorCode::TrustTransitionDenied
    );

    package.version = "3.0.0".into();
    package.artifact_url = Some("https://example.com/v3.tar.gz".into());
    package.trust.content_hash = Some("sha256:verified-v3".into());
    package.created_at += chrono::Duration::seconds(1);
    let current_artifact = VerifiedArtifact {
        url: package.artifact_url.clone().unwrap(),
        content_type: Some("application/gzip".into()),
        size_bytes: 256,
        content_hash: "sha256:verified-v3".into(),
        signature: None,
        public_key: None,
    };
    repo.publish_verified_package(&package, Some(&current_artifact), Some(owner.id))
        .await
        .unwrap();

    let rejected = transition_trust(
        &repo,
        &context,
        &package.id,
        TrustLevel::Verified,
        Some("mixed artifact history is insufficient".into()),
    )
    .await
    .unwrap_err();
    assert_eq!(
        rejected.code,
        k_o_palace::error::PalaceErrorCode::TrustTransitionDenied
    );

    let mut signature_history = package.clone();
    signature_history.id = "signature-history.gene".into();
    signature_history.name = "Signature History".into();
    for (index, signed) in [false, true].into_iter().enumerate() {
        signature_history.version = format!("{}.0.0", index + 1);
        signature_history.created_at += chrono::Duration::seconds(1);
        signature_history.artifact_url = Some(format!(
            "https://example.com/signature-history-{}.tar.gz",
            index + 1
        ));
        signature_history.trust.content_hash = Some(format!("sha256:signed-{index}"));
        let artifact = VerifiedArtifact {
            url: signature_history.artifact_url.clone().unwrap(),
            content_type: Some("application/gzip".into()),
            size_bytes: 256,
            content_hash: signature_history.trust.content_hash.clone().unwrap(),
            signature: signed.then(|| "verified-signature".into()),
            public_key: signed.then(|| "verified-public-key".into()),
        };
        repo.publish_verified_package(&signature_history, Some(&artifact), Some(owner.id))
            .await
            .unwrap();
    }

    let signature_rejected = transition_trust_with_policy(
        &repo,
        &context,
        &signature_history.id,
        TrustLevel::Verified,
        Some("mixed signature history is insufficient".into()),
        true,
    )
    .await
    .unwrap_err();
    assert_eq!(
        signature_rejected.code,
        k_o_palace::error::PalaceErrorCode::TrustTransitionDenied
    );

    let mut all_verified = package;
    all_verified.id = "all-verified-history.gene".into();
    all_verified.name = "All Verified History".into();
    for index in 1..=2 {
        all_verified.version = format!("{index}.0.0");
        all_verified.created_at += chrono::Duration::seconds(1);
        all_verified.artifact_url =
            Some(format!("https://example.com/all-verified-{index}.tar.gz"));
        all_verified.trust.content_hash = Some(format!("sha256:all-verified-{index}"));
        let artifact = VerifiedArtifact {
            url: all_verified.artifact_url.clone().unwrap(),
            content_type: Some("application/gzip".into()),
            size_bytes: 256,
            content_hash: all_verified.trust.content_hash.clone().unwrap(),
            signature: Some("verified-signature".into()),
            public_key: Some("verified-public-key".into()),
        };
        repo.publish_verified_package(&all_verified, Some(&artifact), Some(owner.id))
            .await
            .unwrap();
    }

    let transition = transition_trust_with_policy(
        &repo,
        &context,
        &all_verified.id,
        TrustLevel::Verified,
        Some("every artifact and signature is verified".into()),
        true,
    )
    .await
    .unwrap();
    assert_eq!(transition.to_level, "verified");
}
