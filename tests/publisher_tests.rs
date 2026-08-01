//! Publisher and token management endpoint tests.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use k_o_palace::{app::AppState, config::PalaceConfig, routes::router};
use tower::ServiceExt;

fn test_state() -> AppState {
    let mut config = PalaceConfig::default();
    config.security.allow_public_registration = true;
    AppState::in_memory(config)
}

#[tokio::test]
async fn public_registration_is_disabled_by_default() {
    let app = router(AppState::in_memory(PalaceConfig::default()));
    let resp = app
        .oneshot(
            Request::post("/api/v1/publishers")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"blocked","display_name":"Blocked"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn register_publisher_via_http() {
    let app = router(test_state());
    let body = r#"{"name":"testorg","display_name":"Test Org"}"#;
    let resp = app
        .oneshot(
            Request::post("/api/v1/publishers")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["publisher"]["name"], "testorg");
    assert!(json["token"].as_str().unwrap().starts_with("kop_"));
}

#[tokio::test]
async fn get_publisher_by_name() {
    let state = test_state();
    // Register publisher directly via the auth module
    k_o_palace::auth::register_publisher(&state.repo, "myorg", "My Org", None, None)
        .await
        .unwrap();

    let app = router(state);
    let resp = app
        .oneshot(
            Request::get("/api/v1/publishers/myorg")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["name"], "myorg");
    assert_eq!(json["display_name"], "My Org");
}

#[tokio::test]
async fn create_token_via_http() {
    let state = test_state();
    let (_, token) =
        k_o_palace::auth::register_publisher(&state.repo, "tokentest", "Token Test", None, None)
            .await
            .unwrap();

    let app = router(state);
    let body = r#"{"name":"my-token"}"#;
    let resp = app
        .oneshot(
            Request::post("/api/v1/tokens")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {}", token))
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(json["token"].as_str().unwrap().starts_with("kop_"));
    assert_eq!(json["token_info"]["name"], "my-token");
    // Token hash must never be exposed
    assert!(json.get("token_hash").is_none());
    assert!(json["token_info"].get("token_hash").is_none());
}

#[tokio::test]
async fn list_tokens_never_exposes_hash() {
    let state = test_state();
    let (_, token) =
        k_o_palace::auth::register_publisher(&state.repo, "listtok", "List Tok", None, None)
            .await
            .unwrap();

    // Create an additional token
    k_o_palace::auth::create_api_token(
        &state.repo,
        state
            .repo
            .get_publisher_by_name("listtok")
            .await
            .unwrap()
            .id,
        "extra",
    )
    .await
    .unwrap();

    let app = router(state);
    let resp = app
        .oneshot(
            Request::get("/api/v1/tokens")
                .header("authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let tokens = json.as_array().unwrap();
    assert!(tokens.len() >= 2);
    // No token hash in any response
    for t in tokens {
        assert!(t.get("token_hash").is_none());
    }
}

#[tokio::test]
async fn revoke_token_via_http() {
    let state = test_state();
    let (publisher, token) =
        k_o_palace::auth::register_publisher(&state.repo, "revoketest", "Revoke Test", None, None)
            .await
            .unwrap();

    // Create a second token to revoke
    let (token2_plaintext, _) =
        k_o_palace::auth::create_api_token(&state.repo, publisher.id, "to-revoke")
            .await
            .unwrap();
    let token2_id = state
        .repo
        .list_api_tokens(publisher.id)
        .await
        .unwrap()
        .iter()
        .find(|t| t.name == "to-revoke")
        .map(|t| t.id)
        .unwrap();

    let app = router(state.clone());
    let resp = app
        .oneshot(
            Request::delete(format!("/api/v1/tokens/{}", token2_id))
                .header("authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // The revoked token should fail auth
    let app2 = router(state);
    let resp2 = app2
        .oneshot(
            Request::get("/api/v1/tokens")
                .header("authorization", format!("Bearer {}", token2_plaintext))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp2.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn token_endpoints_require_auth() {
    let app = router(test_state());

    // Create token without auth
    let resp = app
        .clone()
        .oneshot(
            Request::post("/api/v1/tokens")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"test"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // List tokens without auth
    let resp = app
        .oneshot(Request::get("/api/v1/tokens").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn rate_limit_publish() {
    // Use a config with a low publish rate limit
    let mut config = PalaceConfig::default();
    config.security.rate_limit_publish_per_minute = 3;
    let state = AppState::in_memory(config);
    let (_, token) =
        k_o_palace::auth::register_publisher(&state.repo, "rltest", "RL Test", None, None)
            .await
            .unwrap();

    let app = router(state);
    let body = serde_json::json!({
        "id": "test.rl",
        "name": "Test",
        "version": "1.0.0",
        "kind": "gene",
        "description": "test",
        "author": "rltest",
        "license": "MIT",
        "trust": {"level": "community", "signature": null, "public_key": null, "content_hash": null, "publisher": "rltest"},
        "compatibility": {"runtimes": [], "platforms": []},
        "capabilities": {"provides": [], "requires": []},
        "repository": "https://github.com/test/test",
        "artifact_url": "https://github.com/test/test/releases/download/v1.0.0/pkg.tar.gz",
        "homepage": null,
        "tags": [],
        "downloads": 0,
        "success_rate": 0.0,
        "yanked": false,
        "deprecated": null,
        "provenance": null,
        "created_at": "2026-07-31T00:00:00Z",
        "updated_at": "2026-07-31T00:00:00Z"
    }).to_string();

    let mut last_status = StatusCode::OK;
    for i in 0..6 {
        let resp = app
            .clone()
            .oneshot(
                Request::post("/api/v1/packages")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(body.replace("test.rl", &format!("test.rl.{i}"))))
                    .unwrap(),
            )
            .await
            .unwrap();
        last_status = resp.status();
        if last_status == StatusCode::TOO_MANY_REQUESTS {
            break;
        }
    }
    assert_eq!(last_status, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn list_publishers_returns_registered_profiles_in_name_order() {
    let state = test_state();
    k_o_palace::auth::register_publisher(&state.repo, "zeta", "Zeta", None, None)
        .await
        .unwrap();
    k_o_palace::auth::register_publisher(&state.repo, "alpha", "Alpha", None, None)
        .await
        .unwrap();

    let app = router(state);
    let response = app
        .oneshot(
            Request::get("/api/v1/publishers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let publishers: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(publishers[0]["name"], "alpha");
    assert_eq!(publishers[1]["name"], "zeta");
}

#[tokio::test]
async fn public_publisher_response_omits_registration_email() {
    let state = test_state();
    k_o_palace::auth::register_publisher(
        &state.repo,
        "privateorg",
        "Private Org",
        Some("owner@example.com".into()),
        None,
    )
    .await
    .unwrap();

    let app = router(state);
    let response = app
        .oneshot(
            Request::get("/api/v1/publishers/privateorg")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let publisher: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(publisher.get("email").is_none());
}

#[tokio::test]
async fn duplicate_authorization_headers_are_rejected() {
    let state = test_state();
    let (_, token) =
        k_o_palace::auth::register_publisher(&state.repo, "dupeauth", "Dupe Auth", None, None)
            .await
            .unwrap();
    let app = router(state);
    let response = app
        .oneshot(
            Request::get("/api/v1/tokens")
                .header("authorization", format!("Bearer {token}"))
                .header("authorization", "Bearer invalid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn token_id_without_secret_cannot_authenticate() {
    let state = test_state();
    let (publisher, _token) =
        k_o_palace::auth::register_publisher(&state.repo, "opaqueid", "Opaque ID", None, None)
            .await
            .unwrap();
    let token_id = state
        .repo
        .list_api_tokens(publisher.id)
        .await
        .unwrap()
        .into_iter()
        .next()
        .unwrap()
        .id;

    let response = router(state)
        .oneshot(
            Request::get("/api/v1/tokens")
                .header("authorization", format!("Bearer kop_{}", token_id.simple()))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn read_only_token_cannot_manage_tokens() {
    let state = test_state();
    let (_publisher, owner_token) =
        k_o_palace::auth::register_publisher(&state.repo, "readonly", "Read Only", None, None)
            .await
            .unwrap();

    let response = router(state.clone())
        .oneshot(
            Request::post("/api/v1/tokens")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {owner_token}"))
                .body(Body::from(
                    r#"{"name":"read-only","scopes":["packages:read"]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let read_only_token = json["token"].as_str().unwrap();

    let response = router(state)
        .oneshot(
            Request::get("/api/v1/tokens")
                .header("authorization", format!("Bearer {read_only_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn reserved_publisher_namespace_is_rejected() {
    let response = router(test_state())
        .oneshot(
            Request::post("/api/v1/publishers")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"admin","display_name":"Reserved"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn oversized_registration_fields_are_rejected() {
    let cases = [
        serde_json::json!({"name": "longdisplay", "display_name": "x".repeat(129)}),
        serde_json::json!({"name": "longemail", "display_name": "Long Email", "email": format!("{}@example.com", "x".repeat(244))}),
        serde_json::json!({"name": "longwebsite", "display_name": "Long Website", "website": format!("https://example.com/{}", "x".repeat(2030))}),
    ];

    for body in cases {
        let response = router(test_state())
            .oneshot(
                Request::post("/api/v1/publishers")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
async fn token_scope_count_is_bounded_before_delegation() {
    let state = test_state();
    let (_, token) = k_o_palace::auth::register_publisher(
        &state.repo,
        "scopebounds",
        "Scope Bounds",
        None,
        None,
    )
    .await
    .unwrap();
    let app = router(state);

    let response = app
        .oneshot(
            Request::post("/api/v1/tokens")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(
                    serde_json::json!({
                        "name": "too-many",
                        "scopes": vec!["packages:read"; 8],
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
