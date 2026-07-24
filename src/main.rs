//! K-O Palace — Open AI Runtime Registry
//!
//! The sovereign ecosystem for discovering, validating, signing, versioning,
//! evolving, and distributing AI runtime components.
//!
//! Pandora is the flagship runtime. Other runtimes can consume KUBER packages
//! via integration adapters.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

mod types;
mod store;

use types::*;
use store::PackageStore;

// ── App State ──

pub struct AppState {
    pub store: PackageStore,
}

// ── API Handlers ──

async fn list_packages(
    State(state): State<Arc<RwLock<AppState>>>,
    Query(params): Query<ListParams>,
) -> Json<PackageListResponse> {
    let state = state.read().await;
    let packages = state.store.list(&params);
    Json(PackageListResponse {
        total: packages.len(),
        packages,
    })
}

async fn get_package(
    State(state): State<Arc<RwLock<AppState>>>,
    Path(id): Path<String>,
) -> Result<Json<Package>, (StatusCode, String)> {
    let state = state.read().await;
    state
        .store
        .get(&id)
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, format!("Package '{}' not found", id)))
}

async fn search_packages(
    State(state): State<Arc<RwLock<AppState>>>,
    Query(params): Query<SearchParams>,
) -> Json<PackageListResponse> {
    let state = state.read().await;
    let packages = state.store.search(&params.q);
    Json(PackageListResponse {
        total: packages.len(),
        packages,
    })
}

async fn get_featured(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Json<PackageListResponse> {
    let state = state.read().await;
    let packages = state.store.featured();
    Json(PackageListResponse {
        total: packages.len(),
        packages,
    })
}

async fn get_trending(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Json<PackageListResponse> {
    let state = state.read().await;
    let packages = state.store.trending();
    Json(PackageListResponse {
        total: packages.len(),
        packages,
    })
}

async fn get_newest(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Json<PackageListResponse> {
    let state = state.read().await;
    let packages = state.store.newest();
    Json(PackageListResponse {
        total: packages.len(),
        packages,
    })
}

async fn get_categories(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Json<Vec<String>> {
    let state = state.read().await;
    Json(state.store.categories())
}

async fn publish_package(
    State(state): State<Arc<RwLock<AppState>>>,
    Json(pkg): Json<Package>,
) -> Result<Json<Package>, (StatusCode, String)> {
    let mut state = state.write().await;
    state
        .store
        .publish(pkg)
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))
}

async fn health() -> &'static str {
    "ok"
}

// ── Server ──

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let state = Arc::new(RwLock::new(AppState {
        store: PackageStore::new(),
    }));

    // Seed with sample packages
    {
        let mut s = state.write().await;
        s.store.seed_samples();
    }

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/packages", get(list_packages).post(publish_package))
        .route("/api/v1/packages/{id}", get(get_package))
        .route("/api/v1/search", get(search_packages))
        .route("/api/v1/categories", get(get_categories))
        .route("/api/v1/featured", get(get_featured))
        .route("/api/v1/trending", get(get_trending))
        .route("/api/v1/newest", get(get_newest))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = "0.0.0.0:3001";
    println!("K-O Palace listening on http://{addr}");
    println!("  API:    http://{addr}/api/v1/packages");
    println!("  Health: http://{addr}/health");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
