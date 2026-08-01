//! Trust transition and signature tests.

use chrono::Utc;
use k_o_palace::{
    auth::AuthContext,
    models::{Publisher, Role, TrustInfo, TrustLevel},
    trust::{can_transition, transition_trust, verify_signature},
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
        scopes: vec![],
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
