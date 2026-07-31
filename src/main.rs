//! K-O Palace — Open AI Runtime Registry

use k_o_palace::{app::AppState, config::PalaceConfig, routes::router};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,k_o_palace=debug"));
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(filter)
        .init();

    let config = PalaceConfig::from_env();
    let state = AppState::in_memory(config.clone());

    if config.registry.seed_samples {
        if let Err(e) = state.seed_samples().await {
            tracing::error!("failed to seed sample packages: {}", e);
        }
    }

    let addr = config.server.bind_addr;
    let app = router(state);

    tracing::info!("K-O Palace listening on http://{}", addr);
    tracing::info!("  API:    http://{}/api/v1/packages", addr);
    tracing::info!("  Health: http://{}/health", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
        tracing::error!("failed to bind to {}: {}", addr, e);
        e
    })?;

    axum::serve(listener, app)
        .await
        .map_err(|e: std::io::Error| {
            tracing::error!("server error: {}", e);
            Box::<dyn std::error::Error + Send + Sync>::from(e)
        })?;

    Ok(())
}
