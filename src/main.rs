//! K-O Palace — Open AI Runtime Registry

use k_o_palace::{app::AppState, config::PalaceConfig, routes::router};
#[cfg(feature = "reqwest")]
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn validate_startup_config(
    config: &PalaceConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if config.security.replica_count != 1 {
        return Err(
            "PALACE_REPLICA_COUNT must be 1 while rate limiting is process-local".into(),
        );
    }

    if !config.server.bind_addr.ip().is_loopback() {
        if !config.security.behind_tls_proxy {
            return Err(
                "PALACE_BEHIND_TLS_PROXY must be true when binding beyond localhost".into(),
            );
        }

        let public_url = url::Url::parse(&config.server.public_url)
            .map_err(|_| "PALACE_PUBLIC_URL must be a valid URL when binding beyond localhost")?;
        if public_url.scheme() != "https" {
            return Err("PALACE_PUBLIC_URL must use https when binding beyond localhost".into());
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(feature = "reqwest")]
    if std::env::args().nth(1).as_deref() == Some("healthcheck") {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()?;
        let response = client.get("http://127.0.0.1:3001/health").send().await?;
        if response.status().is_success() {
            return Ok(());
        }
        return Err(format!("health endpoint returned {}", response.status()).into());
    }
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,k_o_palace=debug"));
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(filter)
        .init();

    let config = PalaceConfig::from_env();
    validate_startup_config(&config)?;
    #[cfg(feature = "postgres")]
    let state = AppState::postgres(config.clone()).await?;
    #[cfg(not(feature = "postgres"))]
    let state = AppState::in_memory(config.clone());

    if config.registry.seed_samples {
        if let Err(e) = state.seed_samples().await {
            tracing::error!("failed to seed sample packages: {}", e);
        }
    }

    let addr = config.server.bind_addr;
    let app = router(state);

    tracing::info!("K-O Palace listening on {}", config.server.public_url);
    tracing::info!(
        "  API:    {}/api/v1/packages",
        config.server.public_url.trim_end_matches('/')
    );
    tracing::info!(
        "  Health: {}/health",
        config.server.public_url.trim_end_matches('/')
    );

    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
        tracing::error!("failed to bind to {}: {}", addr, e);
        e
    })?;

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e: std::io::Error| {
            tracing::error!("server error: {}", e);
            Box::<dyn std::error::Error + Send + Sync>::from(e)
        })?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!("failed to install Ctrl+C handler: {}", error);
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                tracing::error!("failed to install terminate handler: {}", error);
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received; draining active requests");
}

#[cfg(test)]
mod tests {
    use super::validate_startup_config;
    use k_o_palace::config::PalaceConfig;

    #[test]
    fn rejects_multiple_replicas() {
        let mut config = PalaceConfig::default();
        config.security.replica_count = 2;

        let error = validate_startup_config(&config).unwrap_err();

        assert!(error.to_string().contains("PALACE_REPLICA_COUNT"));
    }

    #[test]
    fn public_bind_requires_tls_proxy_even_when_https_flag_is_disabled() {
        let mut config = PalaceConfig::default();
        config.server.bind_addr = "0.0.0.0:3001".parse().unwrap();
        config.server.public_url = "https://registry.example.com".into();
        config.security.require_https_in_production = false;

        let error = validate_startup_config(&config).unwrap_err();

        assert!(error.to_string().contains("PALACE_BEHIND_TLS_PROXY"));
    }

    #[test]
    fn public_bind_requires_valid_https_public_url() {
        let mut config = PalaceConfig::default();
        config.server.bind_addr = "0.0.0.0:3001".parse().unwrap();
        config.server.public_url = "not-a-url".into();
        config.security.behind_tls_proxy = true;
        config.security.require_https_in_production = false;

        let invalid_error = validate_startup_config(&config).unwrap_err();
        assert!(invalid_error.to_string().contains("PALACE_PUBLIC_URL"));

        config.server.public_url = "http://registry.example.com".into();
        let http_error = validate_startup_config(&config).unwrap_err();
        assert!(http_error.to_string().contains("https"));
    }

    #[test]
    fn public_bind_accepts_tls_proxy_with_https_public_url() {
        let mut config = PalaceConfig::default();
        config.server.bind_addr = "0.0.0.0:3001".parse().unwrap();
        config.server.public_url = "https://registry.example.com".into();
        config.security.behind_tls_proxy = true;

        validate_startup_config(&config).unwrap();
    }
}
