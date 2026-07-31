//! Authorization, token revocation, and malformed manifest tests.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Utc;
use k_o_palace::{
    app::AppState,
    auth::register_publisher,
    config::PalaceConfig,
    models::{CapabilityInfo, CompatibilityInfo, Package, PackageKind, TrustInfo, TrustLevel},
    routes::router,
};
use tower::ServiceExt;

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
        artifact_url: Some(
            "https://github.com/test/test/releases/download/v1.0.0/pkg.tar.gz".into(),
        ),
        homepage: None,
        tags: vec!["test".into()],
        yanked: false,
        provenance: None,
        deprecated: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[tokio::test]
async fn delete_by_non_owner_fails() {
    let state = test_state();
    let (publisher, _token) = register_publisher(&state.repo, "owner", "Owner", None, None)
        .await
        .unwrap();
    let (_other, other_token) = register_publisher(&state.repo, "other", "Other", None, None)
        .await
        .unwrap();

    let mut pkg = valid_pkg("test.gene");
    pkg.trust.publisher = publisher.name.clone();
    state.repo.publish_package(&pkg).await.unwrap();

    let app = router(state);
    let resp = app
        .oneshot(
            Request::delete("/api/v1/packages/test.gene")
                .header("authorization", format!("Bearer {}", other_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn delete_unauthenticated_fails() {
    let state = test_state();
    let (publisher, _token) = register_publisher(&state.repo, "owner", "Owner", None, None)
        .await
        .unwrap();
    let mut pkg = valid_pkg("test.gene");
    pkg.trust.publisher = publisher.name.clone();
    state.repo.publish_package(&pkg).await.unwrap();

    let app = router(state);
    let resp = app
        .oneshot(
            Request::delete("/api/v1/packages/test.gene")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn update_unauthenticated_fails() {
    let state = test_state();
    let (publisher, _token) = register_publisher(&state.repo, "owner", "Owner", None, None)
        .await
        .unwrap();
    let mut pkg = valid_pkg("test.gene");
    pkg.trust.publisher = publisher.name.clone();
    state.repo.publish_package(&pkg).await.unwrap();

    let app = router(state);
    let body = serde_json::to_string(&valid_pkg("test.gene")).unwrap();
    let resp = app
        .oneshot(
            Request::put("/api/v1/packages/test.gene")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn malformed_manifest_rejected() {
    let state = test_state();
    let (_publisher, token) = register_publisher(&state.repo, "pub", "Pub", None, None)
        .await
        .unwrap();

    let app = router(state);
    // Invalid JSON
    let resp = app
        .oneshot(
            Request::post("/api/v1/packages")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {}", token))
                .body(Body::from("{invalid json"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn empty_id_rejected() {
    let state = test_state();
    let (publisher, token) = register_publisher(&state.repo, "pub", "Pub", None, None)
        .await
        .unwrap();

    let mut pkg = valid_pkg("");
    pkg.trust.publisher = publisher.name.clone();
    let body = serde_json::to_string(&pkg).unwrap();

    let app = router(state);
    let resp = app
        .oneshot(
            Request::post("/api/v1/packages")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {}", token))
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn invalid_version_rejected() {
    let state = test_state();
    let (publisher, token) = register_publisher(&state.repo, "pub", "Pub", None, None)
        .await
        .unwrap();

    let mut pkg = valid_pkg("test.gene");
    pkg.version = "not-semver".into();
    pkg.trust.publisher = publisher.name.clone();
    let body = serde_json::to_string(&pkg).unwrap();

    let app = router(state);
    let resp = app
        .oneshot(
            Request::post("/api/v1/packages")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {}", token))
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn reviews_listed_without_auth() {
    let state = test_state();
    let (publisher, _) = register_publisher(&state.repo, "pub", "Pub", None, None)
        .await
        .unwrap();
    let mut pkg = valid_pkg("test.gene");
    pkg.trust.publisher = publisher.name.clone();
    state.repo.publish_package(&pkg).await.unwrap();

    let app = router(state);
    let resp = app
        .oneshot(
            Request::get("/api/v1/packages/test.gene/reviews")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
