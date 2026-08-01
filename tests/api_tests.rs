//! API integration tests.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Utc;
use k_o_palace::{
    app::AppState,
    auth::{create_api_token_with_options, register_publisher},
    config::PalaceConfig,
    models::{
        CapabilityInfo, CompatibilityInfo, Package, PackageKind, Review, ReviewStatus, Role,
        TrustInfo, TrustLevel,
    },
    routes::router,
};
use tower::ServiceExt;
use uuid::Uuid;

fn test_state() -> AppState {
    let config = PalaceConfig::default();
    AppState::in_memory(config)
}

fn valid_pkg(id: impl Into<String>) -> Package {
    Package {
        id: id.into(),
        name: "Test".into(),
        version: "1.0.0".into(),
        kind: PackageKind::Gene,
        description: "test".into(),
        author: "test".into(),
        license: "MIT".into(),
        trust: TrustInfo {
            level: TrustLevel::Community,
            signature: None,
            public_key: None,
            content_hash: None,
            publisher: "testpub".into(),
        },
        capabilities: CapabilityInfo::default(),
        downloads: 0,
        success_rate: 0.0,
        compatibility: CompatibilityInfo::default(),
        repository: Some("https://github.com/test/test".into()),
        artifact_url: None,
        homepage: None,
        tags: vec!["test".into()],
        yanked: false,
        provenance: None,
        deprecated: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn review(package_id: impl Into<String>, reviewer_id: Uuid, rating: i16, comment: &str) -> Review {
    Review {
        id: Uuid::new_v4(),
        package_id: package_id.into(),
        reviewer_id,
        rating,
        comment: Some(comment.into()),
        status: ReviewStatus::Published,
        moderated_by: None,
        moderation_reason: None,
        moderated_at: None,
        created_at: Utc::now(),
    }
}

async fn register_moderator(
    state: &AppState,
    name: &str,
) -> (k_o_palace::models::Publisher, String) {
    let (publisher, _token) = register_publisher(&state.repo, name, name, None, None)
        .await
        .unwrap();
    let publisher = state
        .repo
        .update_publisher_role(publisher.id, Role::Moderator)
        .await
        .unwrap();
    let (token, _) = create_api_token_with_options(
        &state.repo,
        publisher.id,
        format!("{name}-moderation"),
        None,
        vec!["moderation:write".into()],
    )
    .await
    .unwrap();
    (publisher, token)
}

#[tokio::test]
async fn health_ok() {
    let app = router(test_state());
    let resp = app
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let text = String::from_utf8_lossy(&body);
    println!("HEALTH status={status} body={text}");
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn unauthenticated_publish_fails() {
    let app = router(test_state());
    let body = serde_json::to_string(&valid_pkg("test.gene")).unwrap();
    let resp = app
        .clone()
        .oneshot(
            Request::post("/api/v1/packages")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn authenticated_publish_succeeds() {
    let state = test_state();
    let (publisher, token) =
        register_publisher(&state.repo, "testpub", "Test Publisher", None, None)
            .await
            .unwrap();

    let app = router(state);
    let mut pkg = valid_pkg("test.gene");
    pkg.trust.publisher = publisher.name.clone();
    let body = serde_json::to_string(&pkg).unwrap();
    let resp = app
        .clone()
        .oneshot(
            Request::post("/api/v1/packages")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {}", token))
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn forged_verified_metadata_fails() {
    let state = test_state();
    let (publisher, token) =
        register_publisher(&state.repo, "testpub", "Test Publisher", None, None)
            .await
            .unwrap();

    let app = router(state);
    let mut pkg = valid_pkg("test.gene");
    pkg.trust.publisher = publisher.name.clone();
    pkg.trust.level = TrustLevel::Verified; // client forging
    let body = serde_json::to_string(&pkg).unwrap();
    let resp = app
        .clone()
        .oneshot(
            Request::post("/api/v1/packages")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {}", token))
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Fetch and verify normalized to experimental/community
    let get = app
        .clone()
        .oneshot(
            Request::get("/api/v1/packages/test.gene")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(get.into_body(), usize::MAX)
        .await
        .unwrap();
    let fetched: Package = serde_json::from_slice(&bytes).unwrap();
    assert!(!matches!(fetched.trust.level, TrustLevel::Verified));
}

#[tokio::test]
async fn pagination_total_is_accurate() {
    let state = test_state();
    let (publisher, _token) =
        register_publisher(&state.repo, "testpub", "Test Publisher", None, None)
            .await
            .unwrap();

    for i in 0..5 {
        let mut pkg = valid_pkg(format!("test.gene.{}", i));
        pkg.trust.publisher = publisher.name.clone();
        state.repo.publish_package(&pkg).await.unwrap();
    }

    let app = router(state);
    let resp = app
        .clone()
        .oneshot(
            Request::get("/api/v1/packages?limit=2&offset=0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let list: k_o_palace::models::PackageListResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(list.total, 5);
    assert_eq!(list.packages.len(), 2);
    assert_eq!(list.limit, 2);
    assert_eq!(list.offset, 0);
}

#[tokio::test]
async fn authenticated_publish_uses_the_authenticated_publisher_as_author() {
    let state = test_state();
    let (publisher, token) = register_publisher(&state.repo, "owner", "Owner", None, None)
        .await
        .unwrap();
    let app = router(state);
    let mut pkg = valid_pkg("owner/package");
    pkg.author = "client-supplied-author".into();
    pkg.trust.publisher = "client-supplied-publisher".into();

    let response = app
        .oneshot(
            Request::post("/api/v1/packages")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(serde_json::to_string(&pkg).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let package: Package = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(package.author, publisher.name);
    assert_eq!(package.trust.publisher, publisher.name);
}

#[tokio::test]
async fn published_package_versions_cannot_be_updated() {
    let state = test_state();
    let (publisher, token) = register_publisher(&state.repo, "owner", "Owner", None, None)
        .await
        .unwrap();
    let mut package = valid_pkg("immutable");
    package.trust.publisher = publisher.name;
    state.repo.publish_package(&package).await.unwrap();

    let response = router(state)
        .oneshot(
            Request::put("/api/v1/packages/immutable")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(serde_json::to_string(&package).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn artifact_publish_without_digest_is_rejected_before_persistence() {
    let state = test_state();
    let (publisher, token) = register_publisher(&state.repo, "owner", "Owner", None, None)
        .await
        .unwrap();
    let mut package = valid_pkg("unverified-artifact");
    package.trust.publisher = publisher.name;
    package.artifact_url =
        Some("https://github.com/test/test/releases/download/v1.0.0/pkg.tar.gz".into());

    let app = router(state);
    let response = app
        .clone()
        .oneshot(
            Request::post("/api/v1/packages")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(serde_json::to_string(&package).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let missing = app
        .oneshot(
            Request::get("/api/v1/packages/unverified-artifact")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn artifact_publish_with_malformed_digest_is_rejected_before_fetching() {
    let state = test_state();
    let (publisher, token) = register_publisher(&state.repo, "owner", "Owner", None, None)
        .await
        .unwrap();
    let mut package = valid_pkg("malformed-artifact");
    package.trust.publisher = publisher.name;
    package.artifact_url =
        Some("https://github.com/test/test/releases/download/v1.0.0/pkg.tar.gz".into());
    package.trust.content_hash = Some("sha256:not-a-digest".into());

    let app = router(state);
    let response = app
        .clone()
        .oneshot(
            Request::post("/api/v1/packages")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(serde_json::to_string(&package).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let missing = app
        .oneshot(
            Request::get("/api/v1/packages/malformed-artifact")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn missing_download_version_is_rejected() {
    let state = test_state();
    state
        .repo
        .publish_package(&valid_pkg("download.gene"))
        .await
        .unwrap();

    let error = state
        .repo
        .record_download("download.gene", "9.9.9")
        .await
        .unwrap_err();
    assert_eq!(error.code, k_o_palace::error::PalaceErrorCode::NotFound);
}

#[tokio::test]
async fn publisher_cannot_moderate_review() {
    let state = test_state();
    let (owner, owner_token) = register_publisher(&state.repo, "owner", "Owner", None, None)
        .await
        .unwrap();
    let (reviewer, _reviewer_token) =
        register_publisher(&state.repo, "reviewer", "Reviewer", None, None)
            .await
            .unwrap();
    let mut package = valid_pkg("moderation-package");
    package.trust.publisher = owner.name;
    state.repo.publish_package(&package).await.unwrap();

    let seeded_review = review(&package.id, reviewer.id, 4, "looks good");
    state.repo.add_review(&seeded_review).await.unwrap();

    let response = router(state)
        .oneshot(
            Request::patch(format!(
                "/api/v1/packages/{}/reviews/{}",
                package.id, seeded_review.id
            ))
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {owner_token}"))
            .body(Body::from(
                serde_json::json!({
                    "status": "hidden",
                    "reason": "abusive"
                })
                .to_string(),
            ))
            .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn moderator_can_hide_review() {
    let state = test_state();
    let (owner, _owner_token) = register_publisher(&state.repo, "owner", "Owner", None, None)
        .await
        .unwrap();
    let (reviewer, _reviewer_token) =
        register_publisher(&state.repo, "reviewer", "Reviewer", None, None)
            .await
            .unwrap();
    let (moderator, moderator_token) = register_moderator(&state, "moderator").await;

    let mut package = valid_pkg("moderation-target");
    package.trust.publisher = owner.name;
    state.repo.publish_package(&package).await.unwrap();

    let seeded_review = review(&package.id, reviewer.id, 2, "needs work");
    state.repo.add_review(&seeded_review).await.unwrap();

    let response = router(state)
        .oneshot(
            Request::patch(format!(
                "/api/v1/packages/{}/reviews/{}",
                package.id, seeded_review.id
            ))
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {moderator_token}"))
            .body(Body::from(
                serde_json::json!({
                    "status": "hidden",
                    "reason": "personal attack"
                })
                .to_string(),
            ))
            .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let review: Review = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(review.status, ReviewStatus::Hidden);
    assert_eq!(review.moderated_by, Some(moderator.id));
    assert_eq!(review.moderation_reason.as_deref(), Some("personal attack"));
    assert!(review.moderated_at.is_some());
}

#[tokio::test]
async fn list_reviews_excludes_hidden_reviews() {
    let state = test_state();
    let (owner, _owner_token) = register_publisher(&state.repo, "owner", "Owner", None, None)
        .await
        .unwrap();
    let (reviewer_one, _token_one) =
        register_publisher(&state.repo, "reviewer-one", "Reviewer One", None, None)
            .await
            .unwrap();
    let (reviewer_two, _token_two) =
        register_publisher(&state.repo, "reviewer-two", "Reviewer Two", None, None)
            .await
            .unwrap();
    let (moderator, _moderator_token) = register_moderator(&state, "moderator").await;

    let mut package = valid_pkg("public-review-list");
    package.trust.publisher = owner.name;
    state.repo.publish_package(&package).await.unwrap();

    let visible_review = review(&package.id, reviewer_one.id, 5, "visible");
    let hidden_review = review(&package.id, reviewer_two.id, 1, "hidden");
    state.repo.add_review(&visible_review).await.unwrap();
    state.repo.add_review(&hidden_review).await.unwrap();
    state
        .repo
        .moderate_review(
            &package.id,
            hidden_review.id,
            ReviewStatus::Hidden,
            moderator.id,
            Some("spam".into()),
        )
        .await
        .unwrap();

    let response = router(state)
        .oneshot(
            Request::get(format!("/api/v1/packages/{}/reviews", package.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let reviews: Vec<Review> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(reviews.len(), 1);
    assert_eq!(reviews[0].id, visible_review.id);
}

#[tokio::test]
async fn publisher_verification_requires_moderator_and_is_persisted() {
    let state = test_state();
    let (owner, owner_token) = register_publisher(&state.repo, "owner", "Owner", None, None)
        .await
        .unwrap();
    let (_moderator, moderator_token) = register_moderator(&state, "moderator").await;
    let app = router(state.clone());

    let body = serde_json::json!({
        "verified": true,
        "reason": "identity review completed"
    });

    let denied = app
        .clone()
        .oneshot(
            Request::patch("/api/v1/publishers/owner/verification")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {owner_token}"))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let accepted = app
        .oneshot(
            Request::patch("/api/v1/publishers/owner/verification")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {moderator_token}"))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(accepted.status(), StatusCode::OK);

    let response: k_o_palace::models::PublisherVerification = serde_json::from_slice(
        &axum::body::to_bytes(accepted.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(response.publisher_id, owner.id);
    assert!(response.verified);
    assert_eq!(
        response.reason.as_deref(),
        Some("identity review completed")
    );

    let stored = state
        .repo
        .get_publisher_verification(owner.id)
        .await
        .unwrap();
    assert!(stored.verified);
    assert!(stored.verified_at.is_some());
    assert!(stored.verified_by.is_some());
}

#[tokio::test]
async fn dependency_resolution_endpoint_returns_capability_graph() {
    let state = test_state();
    let mut root = valid_pkg("root.package");
    root.capabilities.requires = vec!["cap.database".into()];

    let mut dependency = valid_pkg("database.package");
    dependency.capabilities.provides = vec!["cap.database".into()];
    dependency.compatibility = CompatibilityInfo {
        runtimes: vec!["pandora".into()],
        platforms: vec!["windows".into()],
    };

    state.repo.publish_package(&root).await.unwrap();
    state.repo.publish_package(&dependency).await.unwrap();

    let response = router(state)
        .oneshot(
            Request::get("/api/v1/packages/root.package/resolve?runtime=pandora&platform=windows")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body: k_o_palace::resolve::ResolutionResponse = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(body.complete);
    assert_eq!(body.resolved_dependencies.len(), 1);
    assert_eq!(
        body.resolved_dependencies[0].selected_package_id.as_deref(),
        Some("database.package")
    );
}
