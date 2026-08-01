//! Application configuration.

use std::net::SocketAddr;

/// Application configuration loaded from environment and files.
#[derive(Debug, Clone)]
pub struct PalaceConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub storage: StorageConfig,
    pub security: SecurityConfig,
    pub registry: RegistryConfig,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: SocketAddr,
    pub public_url: String,
    pub request_timeout_seconds: u64,
    pub body_limit_bytes: usize,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub backend: StorageBackend,
    pub local_path: Option<String>,
    pub github_release_api_url: Option<String>,
    pub allowed_hosts: Vec<String>,
    pub max_artifact_size_bytes: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum StorageBackend {
    #[default]
    Local,
    GitHub,
    GitLab,
    Codeberg,
    Oci,
    S3,
    Azure,
    Gcs,
}

#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub cors_origins: Vec<String>,
    pub rate_limit_publish_per_minute: u32,
    pub rate_limit_search_per_minute: u32,
    pub rate_limit_download_per_minute: u32,
    pub rate_limit_auth_per_minute: u32,
    pub token_hash_cost: u32,
    pub require_https_in_production: bool,
    pub max_body_bytes: usize,
    pub request_timeout_secs: u64,
    pub trust_forwarded_headers: bool,
}

#[derive(Debug, Clone)]
pub struct RegistryConfig {
    pub seed_samples: bool,
    pub default_trust_level: String,
    pub require_signatures_for_verified_and_above: bool,
    pub max_redirects: u32,
}

impl Default for PalaceConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                bind_addr: "127.0.0.1:3001"
                    .parse()
                    .expect("valid localhost socket address"),
                public_url: "http://127.0.0.1:3001".into(),
                request_timeout_seconds: 30,
                body_limit_bytes: 16 * 1024 * 1024,
            },
            database: DatabaseConfig {
                url: "postgres://kopalace:kopalace@localhost:5432/kopalace".into(),
                max_connections: 10,
            },
            storage: StorageConfig {
                backend: StorageBackend::Local,
                local_path: Some("./artifacts".into()),
                github_release_api_url: None,
                allowed_hosts: vec![],
                max_artifact_size_bytes: 512 * 1024 * 1024,
            },
            security: SecurityConfig {
                cors_origins: vec![],
                rate_limit_publish_per_minute: 10,
                rate_limit_search_per_minute: 120,
                rate_limit_download_per_minute: 240,
                rate_limit_auth_per_minute: 10,
                token_hash_cost: bcrypt::DEFAULT_COST,
                require_https_in_production: true,
                max_body_bytes: 16 * 1024 * 1024,
                request_timeout_secs: 30,
                trust_forwarded_headers: false,
            },
            registry: RegistryConfig {
                seed_samples: false,
                default_trust_level: "experimental".into(),
                require_signatures_for_verified_and_above: true,
                max_redirects: 2,
            },
        }
    }
}

impl StorageBackend {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "local" => Some(Self::Local),
            "github" | "github_release" => Some(Self::GitHub),
            "gitlab" => Some(Self::GitLab),
            "codeberg" => Some(Self::Codeberg),
            "oci" => Some(Self::Oci),
            "s3" => Some(Self::S3),
            "azure" => Some(Self::Azure),
            "gcs" => Some(Self::Gcs),
            _ => None,
        }
    }
}
impl PalaceConfig {
    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        if let Ok(addr) = std::env::var("PALACE_BIND") {
            if let Ok(socket) = addr.parse() {
                cfg.server.bind_addr = socket;
            }
        }
        if let Ok(url) = std::env::var("DATABASE_URL") {
            cfg.database.url = url;
        }
        if let Ok(value) = std::env::var("PALACE_DB_MAX_CONNECTIONS") {
            if let Ok(value) = value.parse::<u32>() {
                cfg.database.max_connections = value.max(1);
            }
        }
        if let Ok(url) = std::env::var("PALACE_PUBLIC_URL") {
            cfg.server.public_url = url;
        }
        if let Ok(cors) = std::env::var("PALACE_CORS_ORIGINS") {
            cfg.security.cors_origins = cors.split(',').map(|s| s.trim().to_string()).collect();
        }
        if let Ok(hosts) = std::env::var("PALACE_ALLOWED_HOSTS") {
            cfg.storage.allowed_hosts = hosts
                .split(',')
                .map(|s| s.trim().to_ascii_lowercase())
                .filter(|s| !s.is_empty())
                .collect();
        }
        if let Ok(backend) = std::env::var("PALACE_STORAGE_BACKEND") {
            if let Some(value) = StorageBackend::parse(&backend) {
                cfg.storage.backend = value;
            }
        }
        if let Ok(path) = std::env::var("PALACE_STORAGE_LOCAL_PATH") {
            cfg.storage.local_path = Some(path);
        }
        if let Ok(url) = std::env::var("PALACE_GITHUB_RELEASE_API_URL") {
            cfg.storage.github_release_api_url = Some(url);
        }
        if let Ok(size) = std::env::var("PALACE_MAX_ARTIFACT_SIZE_BYTES") {
            if let Ok(value) = size.parse() {
                cfg.storage.max_artifact_size_bytes = value;
            }
        }
        if let Ok(size) = std::env::var("PALACE_MAX_BODY_BYTES") {
            if let Ok(value) = size.parse() {
                cfg.security.max_body_bytes = value;
                cfg.server.body_limit_bytes = value;
            }
        }
        if let Ok(seconds) = std::env::var("PALACE_REQUEST_TIMEOUT_SECS") {
            if let Ok(value) = seconds.parse() {
                cfg.security.request_timeout_secs = value;
                cfg.server.request_timeout_seconds = value;
            }
        }
        if let Ok(value) = std::env::var("PALACE_RATE_LIMIT_PUBLISH_PER_MINUTE") {
            if let Ok(value) = value.parse() {
                cfg.security.rate_limit_publish_per_minute = value;
            }
        }
        if let Ok(value) = std::env::var("PALACE_RATE_LIMIT_SEARCH_PER_MINUTE") {
            if let Ok(value) = value.parse() {
                cfg.security.rate_limit_search_per_minute = value;
            }
        }
        if let Ok(value) = std::env::var("PALACE_RATE_LIMIT_DOWNLOAD_PER_MINUTE") {
            if let Ok(value) = value.parse() {
                cfg.security.rate_limit_download_per_minute = value;
            }
        }
        if let Ok(value) = std::env::var("PALACE_RATE_LIMIT_AUTH_PER_MINUTE") {
            if let Ok(value) = value.parse() {
                cfg.security.rate_limit_auth_per_minute = value;
            }
        }
        if let Ok(value) = std::env::var("PALACE_TRUST_PROXY_HEADERS") {
            cfg.security.trust_forwarded_headers =
                value == "1" || value.eq_ignore_ascii_case("true");
        }
        if let Ok(seed) = std::env::var("PALACE_SEED_SAMPLES") {
            cfg.registry.seed_samples = seed == "1" || seed.eq_ignore_ascii_case("true");
        }
        cfg
    }
}
