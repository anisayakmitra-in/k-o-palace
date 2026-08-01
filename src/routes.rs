//! HTTP route handlers for K-O Palace.

use crate::{
    app::AppState,
    auth::{authenticate, create_api_token, register_publisher, AuthContext},
    error::{PalaceError, PalaceErrorCode, PalaceResult},
    models::{
        ListParams, Package, PackageListResponse, PublisherRegisterRequest,
        PublisherRegisterResponse, PublisherResponse, Review, ReviewRequest, SearchParams,
        TokenCreateRequest, TokenCreateResponse, TokenResponse, VersionListResponse,
    },
    pagination::Pagination,
    repository::PackageFilters,
    request_id::request_id_middleware,
    search::rank_results,
    validation::{normalize_trust_level, validate_package},
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Redirect,
    routing::{delete, get},
    Json, Router,
};
use std::sync::Arc;
use std::time::Duration;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    limit::RequestBodyLimitLayer,
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

pub fn router(state: AppState) -> Router {
    let cors = if state.config.security.cors_origins.is_empty() {
        CorsLayer::new()
    } else {
        let origins = state
            .config
            .security
            .cors_origins
            .clone()
            .into_iter()
            .filter_map(|o| o.parse().ok())
            .collect::<Vec<_>>();
        CorsLayer::new().allow_origin(AllowOrigin::list(origins))
    };

    let body_limit = RequestBodyLimitLayer::new(state.config.security.max_body_bytes);
    let timeout = TimeoutLayer::with_status_code(
        axum::http::StatusCode::REQUEST_TIMEOUT,
        Duration::from_secs(state.config.security.request_timeout_secs),
    );

    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/version", get(version))
        // Publisher management
        .route(
            "/api/v1/publishers",
            get(list_publishers).post(register_publisher_handler),
        )
        .route("/api/v1/publishers/{name}", get(get_publisher))
        // Token management
        .route(
            "/api/v1/tokens",
            get(list_tokens).post(create_token_handler),
        )
        .route("/api/v1/tokens/{id}", delete(revoke_token_handler))
        // Package management
        .route("/api/v1/packages", get(list_packages).post(publish_package))
        .route(
            "/api/v1/packages/{id}",
            get(get_package).put(update_package).delete(delete_package),
        )
        .route("/api/v1/packages/{id}/versions", get(list_versions))
        .route("/api/v1/packages/{id}/versions/{version}", get(get_version))
        .route("/api/v1/packages/{id}/download", get(download_package))
        .route(
            "/api/v1/packages/{id}/reviews",
            get(list_reviews).post(add_review),
        )
        .route("/api/v1/search", get(search_packages))
        .route("/api/v1/categories", get(get_categories))
        .route("/api/v1/featured", get(get_featured))
        .route("/api/v1/trending", get(get_trending))
        .route("/api/v1/newest", get(get_newest))
        .route("/api/v1/runtimes", get(get_runtimes))
        .layer(body_limit)
        .layer(timeout)
        .layer(cors)
        .layer(axum::middleware::from_fn(request_id_middleware))
        .layer(TraceLayer::new_for_http())
        .with_state(Arc::new(state))
}

async fn health(State(state): State<Arc<AppState>>) -> StatusCode {
    if state.repo.is_healthy().await {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

async fn ready(State(state): State<Arc<AppState>>) -> StatusCode {
    health(State(state)).await
}

async fn version() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "name": "k-o-palace",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn list_packages(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListParams>,
) -> PalaceResult<Json<PackageListResponse>> {
    let pagination = Pagination::new(params.limit, params.offset)?;
    let filters = PackageFilters {
        q: params.q,
        kind: params.kind,
        category: params.category,
        runtime: params.runtime,
    };
    let (total, packages) = state.repo.list_packages(filters, pagination).await?;
    Ok(Json(PackageListResponse {
        total,
        limit: pagination.limit,
        offset: pagination.offset,
        packages,
    }))
}

async fn get_package(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> PalaceResult<Json<Package>> {
    Ok(Json(state.repo.get_package(&id).await?))
}

async fn list_versions(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> PalaceResult<Json<VersionListResponse>> {
    let versions = state.repo.list_versions(&id).await?;
    Ok(Json(VersionListResponse {
        total: versions.len(),
        versions: versions.into_iter().map(|v| v.version).collect(),
    }))
}

async fn get_version(
    State(state): State<Arc<AppState>>,
    Path((id, version)): Path<(String, String)>,
) -> PalaceResult<Json<Package>> {
    Ok(Json(state.repo.get_package_version(&id, &version).await?))
}

async fn search_packages(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> PalaceResult<Json<PackageListResponse>> {
    // Rate limit
    let rl_key = "search";
    if let Err(retry) = state.rate_limiters.search.check(rl_key).await {
        return Err(PalaceError::new(
            PalaceErrorCode::RateLimited,
            format!("rate limit exceeded, retry after {retry}s"),
        ));
    }

    if params.q.trim().is_empty() {
        return Err(PalaceError::new(
            PalaceErrorCode::BadRequest,
            "search query cannot be empty",
        ));
    }
    if params.q.len() > 256 {
        return Err(PalaceError::new(
            PalaceErrorCode::BadRequest,
            "search query too long",
        ));
    }
    let pagination = Pagination::new(params.limit, params.offset)?;
    let (total, mut packages) = state.repo.search(&params.q, pagination).await?;
    rank_results(&params.q, &mut packages);
    Ok(Json(PackageListResponse {
        total,
        limit: pagination.limit,
        offset: pagination.offset,
        packages,
    }))
}

async fn get_categories(State(state): State<Arc<AppState>>) -> PalaceResult<Json<Vec<String>>> {
    Ok(Json(state.repo.categories().await?))
}

async fn get_featured(
    State(state): State<Arc<AppState>>,
) -> PalaceResult<Json<PackageListResponse>> {
    let packages = state.repo.featured(10).await?;
    Ok(Json(PackageListResponse {
        total: packages.len(),
        limit: 10,
        offset: 0,
        packages,
    }))
}

async fn get_trending(
    State(state): State<Arc<AppState>>,
) -> PalaceResult<Json<PackageListResponse>> {
    let packages = state.repo.trending(20).await?;
    Ok(Json(PackageListResponse {
        total: packages.len(),
        limit: 20,
        offset: 0,
        packages,
    }))
}

async fn get_newest(State(state): State<Arc<AppState>>) -> PalaceResult<Json<PackageListResponse>> {
    let packages = state.repo.newest(20).await?;
    Ok(Json(PackageListResponse {
        total: packages.len(),
        limit: 20,
        offset: 0,
        packages,
    }))
}

async fn get_runtimes(State(state): State<Arc<AppState>>) -> PalaceResult<Json<Vec<String>>> {
    Ok(Json(state.repo.runtimes().await?))
}

async fn publish_package(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(mut pkg): Json<Package>,
) -> PalaceResult<(StatusCode, Json<Package>)> {
    // Rate limit
    let rl_key = "publish"; // Simplified: per-endpoint, not per-client
    if let Err(retry) = state.rate_limiters.publish.check(rl_key).await {
        return Err(PalaceError::new(
            PalaceErrorCode::RateLimited,
            format!("rate limit exceeded, retry after {retry}s"),
        ));
    }

    let auth = authenticate_header(&state, &headers).await?;
    if !auth.can_publish() {
        return Err(PalaceError::new(
            PalaceErrorCode::Forbidden,
            "insufficient role to publish",
        ));
    }

    validate_package(&pkg)?;

    pkg.trust.level = normalize_trust_level(Some(pkg.trust.level.as_str()));
    pkg.author = auth.publisher.name.clone();
    pkg.trust.publisher = auth.publisher.name.clone();

    if !auth.owns(&pkg.trust.publisher) {
        return Err(PalaceError::new(
            PalaceErrorCode::Forbidden,
            "cannot publish under another publisher",
        ));
    }

    if pkg.artifact_url.is_some() && pkg.trust.content_hash.is_none() {
        return Err(PalaceError::new(
            PalaceErrorCode::HashMismatch,
            "artifact-bearing packages require a content_hash",
        ));
    }

    if let Some(artifact_url) = &pkg.artifact_url {
        crate::artifact::fetch_and_verify_package_artifact(artifact_url, &pkg.trust, &state.config)
            .await?;
    }

    let package = state.repo.publish_package(&pkg).await?;
    Ok((StatusCode::CREATED, Json(package)))
}

async fn update_package(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
    Json(mut pkg): Json<Package>,
) -> PalaceResult<Json<Package>> {
    let auth = authenticate_header(&state, &headers).await?;
    let existing = state.repo.get_package(&id).await?;
    if !auth.owns(&existing.trust.publisher) {
        return Err(PalaceError::new(
            PalaceErrorCode::Forbidden,
            "only the publisher or admin can update this package",
        ));
    }

    validate_package(&pkg)?;
    pkg.author = existing.author.clone();
    pkg.trust.publisher = existing.trust.publisher.clone();
    pkg.trust.level = existing.trust.level.clone();
    pkg.id = id;
    Ok(Json(state.repo.update_package(&pkg).await?))
}

async fn delete_package(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> PalaceResult<StatusCode> {
    let auth = authenticate_header(&state, &headers).await?;
    let existing = state.repo.get_package(&id).await?;
    if !auth.owns(&existing.trust.publisher) && !auth.can_moderate() {
        return Err(PalaceError::new(
            PalaceErrorCode::Forbidden,
            "only the publisher or moderator can delete this package",
        ));
    }

    state.repo.delete_package(&id, auth.publisher.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn download_package(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> PalaceResult<Redirect> {
    // Rate limit
    let rl_key = "download";
    if let Err(retry) = state.rate_limiters.download.check(rl_key).await {
        return Err(PalaceError::new(
            PalaceErrorCode::RateLimited,
            format!("rate limit exceeded, retry after {retry}s"),
        ));
    }

    let package = state.repo.get_package(&id).await?;
    let artifact_url = package.artifact_url.ok_or_else(|| {
        PalaceError::new(
            PalaceErrorCode::NotFound,
            format!("Package '{}' has no published artifact", id),
        )
    })?;

    // Validate the artifact URL is HTTPS and allowlisted
    crate::artifact::validate_artifact_url(&artifact_url, &state.config)?;

    // Record download
    state.repo.record_download(&id, &package.version).await?;

    // Redirect to the artifact URL
    Ok(Redirect::temporary(&artifact_url))
}

async fn list_reviews(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> PalaceResult<Json<Vec<Review>>> {
    Ok(Json(state.repo.list_reviews(&id).await?))
}

async fn add_review(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<ReviewRequest>,
) -> PalaceResult<(StatusCode, Json<Review>)> {
    let auth = authenticate_header(&state, &headers).await?;
    if req.rating < 1 || req.rating > 5 {
        return Err(PalaceError::new(
            PalaceErrorCode::BadRequest,
            "rating must be between 1 and 5",
        ));
    }

    let review = Review {
        id: uuid::Uuid::new_v4(),
        package_id: id,
        reviewer_id: auth.publisher.id,
        rating: req.rating,
        comment: req.comment,
        created_at: chrono::Utc::now(),
    };
    let review = state.repo.add_review(&review).await?;
    Ok((StatusCode::CREATED, Json(review)))
}

async fn authenticate_header(
    state: &Arc<AppState>,
    headers: &axum::http::HeaderMap,
) -> PalaceResult<AuthContext> {
    let mut values = headers.get_all(axum::http::header::AUTHORIZATION).iter();
    let auth = values.next().and_then(|value| value.to_str().ok());
    if values.next().is_some() {
        return Err(PalaceError::new(
            PalaceErrorCode::Unauthorized,
            "multiple authorization headers are not allowed",
        ));
    }
    authenticate(&state.repo, auth).await
}

// ── Publisher Management Handlers ──

async fn register_publisher_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PublisherRegisterRequest>,
) -> PalaceResult<(StatusCode, Json<PublisherRegisterResponse>)> {
    let (publisher, token) = register_publisher(
        &state.repo,
        &req.name,
        &req.display_name,
        req.email.clone(),
        req.website.clone(),
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(PublisherRegisterResponse {
            publisher: publisher.into(),
            token,
        }),
    ))
}

async fn get_publisher(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> PalaceResult<Json<PublisherResponse>> {
    let publisher = state.repo.get_publisher_by_name(&name).await?;
    Ok(Json(publisher.into()))
}

async fn list_publishers(
    State(state): State<Arc<AppState>>,
) -> PalaceResult<Json<Vec<PublisherResponse>>> {
    let publishers = state.repo.list_publishers().await?;
    Ok(Json(publishers.into_iter().map(Into::into).collect()))
}

// ── Token Management Handlers ──

async fn create_token_handler(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(req): Json<TokenCreateRequest>,
) -> PalaceResult<(StatusCode, Json<TokenCreateResponse>)> {
    let auth = authenticate_header(&state, &headers).await?;

    let (plaintext, token) = create_api_token(&state.repo, auth.publisher.id, &req.name).await?;

    // Record audit event
    let audit = crate::models::AuditEvent {
        id: uuid::Uuid::new_v4(),
        event_type: "token.created".into(),
        actor_id: Some(auth.publisher.id),
        package_id: None,
        details: Some(serde_json::json!({"token_name": req.name})),
        created_at: chrono::Utc::now(),
    };
    let _ = state.repo.record_audit_event(&audit).await;

    Ok((
        StatusCode::CREATED,
        Json(TokenCreateResponse {
            token: plaintext,
            token_info: token.into(),
        }),
    ))
}

async fn list_tokens(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> PalaceResult<Json<Vec<TokenResponse>>> {
    let auth = authenticate_header(&state, &headers).await?;
    let tokens = state.repo.list_api_tokens(auth.publisher.id).await?;
    Ok(Json(tokens.into_iter().map(Into::into).collect()))
}

async fn revoke_token_handler(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<uuid::Uuid>,
) -> PalaceResult<StatusCode> {
    let auth = authenticate_header(&state, &headers).await?;

    // Verify ownership: the token must belong to the authenticated publisher
    let tokens = state.repo.list_api_tokens(auth.publisher.id).await?;
    if !tokens.iter().any(|t| t.id == id) {
        return Err(PalaceError::new(
            PalaceErrorCode::Forbidden,
            "token does not belong to authenticated publisher",
        ));
    }

    state.repo.revoke_api_token(id).await?;

    // Record audit event
    let audit = crate::models::AuditEvent {
        id: uuid::Uuid::new_v4(),
        event_type: "token.revoked".into(),
        actor_id: Some(auth.publisher.id),
        package_id: None,
        details: Some(serde_json::json!({"token_id": id})),
        created_at: chrono::Utc::now(),
    };
    let _ = state.repo.record_audit_event(&audit).await;

    Ok(StatusCode::NO_CONTENT)
}
