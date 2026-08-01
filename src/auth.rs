//! Authentication, authorization, and API token management.

use crate::error::{PalaceError, PalaceErrorCode, PalaceResult};
use crate::models::{ApiToken, Publisher, Role};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::Utc;
use uuid::Uuid;

/// Scopes supported by API tokens.
pub const SUPPORTED_SCOPES: &[&str] = &[
    "packages:read",
    "packages:publish",
    "packages:write",
    "tokens:manage",
    "reviews:write",
    "moderation:write",
    "admin:write",
];

/// Scopes issued to a publisher's initial token.
pub const DEFAULT_PUBLISHER_SCOPES: &[&str] = &[
    "packages:read",
    "packages:publish",
    "packages:write",
    "tokens:manage",
    "reviews:write",
];

const MAX_DISPLAY_NAME_LEN: usize = 128;
const MAX_EMAIL_LEN: usize = 254;
const MAX_WEBSITE_LEN: usize = 2_048;
const MAX_TOKEN_SCOPES: usize = SUPPORTED_SCOPES.len();
const MAX_TOKEN_SCOPE_BYTES: usize = 1_024;

/// Authenticated context extracted from an API token.
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub publisher: Publisher,
    pub token_id: Uuid,
    pub scopes: Vec<String>,
}

impl AuthContext {
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes
            .iter()
            .any(|value| value == "*" || value == scope)
    }

    pub fn can_publish(&self) -> bool {
        self.publisher.role.can_publish() && self.has_scope("packages:publish")
    }

    pub fn can_moderate(&self) -> bool {
        self.publisher.role.can_moderate() && self.has_scope("moderation:write")
    }

    pub fn can_administer(&self) -> bool {
        self.publisher.role.can_administer() && self.has_scope("admin:write")
    }

    pub fn can_issue_scope(&self, scope: &str) -> bool {
        self.has_scope(scope)
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
    let token_id = crate::repository::token_id_from_opaque(token).ok_or_else(|| {
        PalaceError::new(PalaceErrorCode::Unauthorized, "invalid or revoked token")
    })?;
    let api_token = repo.get_api_token_by_id(token_id).await?;
    if !constant_time_verify(token, &api_token.token_hash) {
        return Err(PalaceError::new(
            PalaceErrorCode::Unauthorized,
            "invalid or revoked token",
        ));
    }

    if let Some(expires) = api_token.expires_at {
        if expires <= Utc::now() {
            return Err(PalaceError::new(
                PalaceErrorCode::Unauthorized,
                "token expired",
            ));
        }
    }

    repo.touch_api_token(api_token.id).await?;
    let publisher = repo.get_publisher_by_id(api_token.publisher_id).await?;
    Ok(AuthContext {
        publisher,
        token_id: api_token.id,
        scopes: api_token.scopes,
    })
}

fn validate_scopes(scopes: &[String]) -> PalaceResult<Vec<String>> {
    if scopes.len() > MAX_TOKEN_SCOPES {
        return Err(PalaceError::new(
            PalaceErrorCode::BadRequest,
            format!("token may contain at most {MAX_TOKEN_SCOPES} scopes"),
        ));
    }
    let total_bytes = scopes
        .iter()
        .try_fold(0usize, |total, scope| total.checked_add(scope.len()))
        .ok_or_else(|| {
            PalaceError::new(
                PalaceErrorCode::BadRequest,
                "token scope payload is too large",
            )
        })?;
    if total_bytes > MAX_TOKEN_SCOPE_BYTES {
        return Err(PalaceError::new(
            PalaceErrorCode::BadRequest,
            format!("token scopes may contain at most {MAX_TOKEN_SCOPE_BYTES} bytes"),
        ));
    }

    let mut normalized = Vec::with_capacity(scopes.len());
    for scope in scopes {
        if !SUPPORTED_SCOPES.contains(&scope.as_str()) {
            return Err(PalaceError::new(
                PalaceErrorCode::BadRequest,
                format!("unsupported token scope: {scope}"),
            ));
        }
        if !normalized.contains(scope) {
            normalized.push(scope.clone());
        }
    }
    Ok(normalized)
}

fn prepare_api_token(
    publisher_id: Uuid,
    name: impl Into<String>,
    expires_at: Option<chrono::DateTime<Utc>>,
    scopes: Vec<String>,
    hash_cost: u32,
) -> PalaceResult<(String, ApiToken)> {
    let name = name.into();
    if name.trim().is_empty() || name.len() > 256 {
        return Err(PalaceError::new(
            PalaceErrorCode::BadRequest,
            "token name must be 1-256 characters",
        ));
    }
    if expires_at.is_some_and(|value| value <= Utc::now()) {
        return Err(PalaceError::new(
            PalaceErrorCode::BadRequest,
            "token expiry must be in the future",
        ));
    }
    let scopes = validate_scopes(&scopes)?;
    let id = Uuid::new_v4();
    let plaintext = format!("kop_{}_{}", id.simple(), Uuid::new_v4().simple());
    let token = ApiToken {
        id,
        publisher_id,
        name,
        token_hash: hash_token_plaintext_with_cost(&plaintext, hash_cost),
        created_at: Utc::now(),
        revoked_at: None,
        expires_at,
        scopes,
    };
    Ok((plaintext, token))
}

/// Generate a token with explicit lifecycle and scope controls.
pub async fn create_api_token_with_options(
    repo: &crate::repository::PackageRepository,
    publisher_id: Uuid,
    name: impl Into<String>,
    expires_at: Option<chrono::DateTime<Utc>>,
    scopes: Vec<String>,
) -> PalaceResult<(String, ApiToken)> {
    let (plaintext, token) =
        prepare_api_token(publisher_id, name, expires_at, scopes, DEFAULT_COST)?;
    repo.create_api_token(&token).await?;
    Ok((plaintext, token))
}

/// Generate a token and persist it with its audit event atomically where supported.
pub async fn create_api_token_with_options_and_audit(
    repo: &crate::repository::PackageRepository,
    publisher_id: Uuid,
    name: impl Into<String>,
    expires_at: Option<chrono::DateTime<Utc>>,
    scopes: Vec<String>,
    hash_cost: u32,
    audit: &crate::models::AuditEvent,
) -> PalaceResult<(String, ApiToken)> {
    let (plaintext, token) = prepare_api_token(publisher_id, name, expires_at, scopes, hash_cost)?;
    repo.create_api_token_with_audit(&token, audit).await?;
    Ok((plaintext, token))
}

/// Generate a new API token with the default publisher scopes.
pub async fn create_api_token(
    repo: &crate::repository::PackageRepository,
    publisher_id: Uuid,
    name: impl Into<String>,
) -> PalaceResult<(String, ApiToken)> {
    create_api_token_with_options(
        repo,
        publisher_id,
        name,
        None,
        DEFAULT_PUBLISHER_SCOPES
            .iter()
            .map(|scope| (*scope).into())
            .collect(),
    )
    .await
}

/// Hash a plaintext token for storage using bcrypt.
pub fn hash_token_plaintext(token: &str) -> String {
    hash_token_plaintext_with_cost(token, DEFAULT_COST)
}

pub fn hash_token_plaintext_with_cost(token: &str, cost: u32) -> String {
    hash(token.as_bytes(), cost.clamp(4, 31)).expect("bcrypt hash should not fail")
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
    let display_name = display_name.into();
    validate_publisher_registration(&name, &display_name, email.as_deref(), website.as_deref())?;
    let publisher = Publisher {
        id: Uuid::new_v4(),
        name: name.clone(),
        display_name,
        email,
        website,
        role: Role::Publisher,
        created_at: Utc::now(),
    };
    let publisher = repo.create_publisher(&publisher).await?;
    let (token, _) = create_api_token(repo, publisher.id, format!("{name}-default")).await?;
    Ok((publisher, token))
}

fn validate_publisher_registration(
    name: &str,
    display_name: &str,
    email: Option<&str>,
    website: Option<&str>,
) -> PalaceResult<()> {
    crate::identity::validate_namespace(name)?;
    if display_name.len() > MAX_DISPLAY_NAME_LEN {
        return Err(PalaceError::new(
            PalaceErrorCode::BadRequest,
            format!("display name must be at most {MAX_DISPLAY_NAME_LEN} bytes"),
        ));
    }
    if email.is_some_and(|value| value.len() > MAX_EMAIL_LEN) {
        return Err(PalaceError::new(
            PalaceErrorCode::BadRequest,
            format!("email must be at most {MAX_EMAIL_LEN} bytes"),
        ));
    }
    if website.is_some_and(|value| value.len() > MAX_WEBSITE_LEN) {
        return Err(PalaceError::new(
            PalaceErrorCode::BadRequest,
            format!("website must be at most {MAX_WEBSITE_LEN} bytes"),
        ));
    }
    Ok(())
}
