//! Authentication, authorization, and API token management.

use crate::error::{PalaceError, PalaceErrorCode, PalaceResult};
use crate::models::{ApiToken, Publisher, Role};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::Utc;
use uuid::Uuid;

/// Authenticated context extracted from an API token.
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub publisher: Publisher,
    pub token_id: Uuid,
}

impl AuthContext {
    pub fn can_publish(&self) -> bool {
        self.publisher.role.can_publish()
    }

    pub fn can_moderate(&self) -> bool {
        self.publisher.role.can_moderate()
    }

    pub fn can_administer(&self) -> bool {
        self.publisher.role.can_administer()
    }

    pub fn owns(&self, package_publisher: &str) -> bool {
        self.publisher.name == package_publisher || self.can_administer()
    }
}

/// Authorization header prefix.
const BEARER_PREFIX: &str = "Bearer ";

/// Extract and verify a bearer token.
pub async fn authenticate(
    repo: &crate::repository::PackageRepository,
    auth_header: Option<&str>,
) -> PalaceResult<AuthContext> {
    let header = auth_header.ok_or_else(|| {
        PalaceError::new(
            PalaceErrorCode::Unauthorized,
            "missing authorization header",
        )
    })?;
    if !header.starts_with(BEARER_PREFIX) {
        return Err(PalaceError::new(
            PalaceErrorCode::Unauthorized,
            "invalid authorization scheme",
        ));
    }
    let token = &header[BEARER_PREFIX.len()..];
    verify_token(repo, token).await
}

async fn verify_token(
    repo: &crate::repository::PackageRepository,
    token: &str,
) -> PalaceResult<AuthContext> {
    let api_token = repo.get_api_token_by_plaintext(token).await?;

    if let Some(expires) = api_token.expires_at {
        if expires < Utc::now() {
            return Err(PalaceError::new(
                PalaceErrorCode::Unauthorized,
                "token expired",
            ));
        }
    }

    let publisher = repo.get_publisher_by_id(api_token.publisher_id).await?;
    Ok(AuthContext {
        publisher,
        token_id: api_token.id,
    })
}

/// Generate a new API token, returning the plaintext and the stored record.
pub async fn create_api_token(
    repo: &crate::repository::PackageRepository,
    publisher_id: Uuid,
    name: impl Into<String>,
) -> PalaceResult<(String, ApiToken)> {
    let plaintext = format!("kop_{}", Uuid::new_v4().to_string().replace('-', ""));
    let token_hash = hash_token_plaintext(&plaintext);
    let token = ApiToken {
        id: Uuid::new_v4(),
        publisher_id,
        name: name.into(),
        token_hash,
        created_at: Utc::now(),
        revoked_at: None,
        expires_at: None,
    };
    repo.create_api_token(&token).await?;
    Ok((plaintext, token))
}

/// Hash a plaintext token for storage using bcrypt.
pub fn hash_token_plaintext(token: &str) -> String {
    hash(token.as_bytes(), DEFAULT_COST).expect("bcrypt hash should not fail")
}

/// Constant-time-ish verification of a token against a stored hash.
pub fn constant_time_verify(token: &str, hash: &str) -> bool {
    verify(token.as_bytes(), hash).unwrap_or(false)
}

/// Register a new publisher.
pub async fn register_publisher(
    repo: &crate::repository::PackageRepository,
    name: impl Into<String>,
    display_name: impl Into<String>,
    email: Option<String>,
    website: Option<String>,
) -> PalaceResult<(Publisher, String)> {
    let name = name.into();
    if name.is_empty() || name.len() > 64 {
        return Err(PalaceError::new(
            PalaceErrorCode::ValidationFailed,
            "publisher name must be 1-64 characters",
        ));
    }
    let publisher = Publisher {
        id: Uuid::new_v4(),
        name: name.clone(),
        display_name: display_name.into(),
        email,
        website,
        role: Role::Publisher,
        created_at: Utc::now(),
    };
    let publisher = repo.create_publisher(&publisher).await?;
    let (token, _) = create_api_token(repo, publisher.id, format!("{name}-default")).await?;
    Ok((publisher, token))
}
