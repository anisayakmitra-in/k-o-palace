//! API integration tests.

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
