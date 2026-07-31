//! Structured errors with stable codes and JSON responses.

use axum::{
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Stable error codes for clients and logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PalaceErrorCode {
    BadRequest,
    ValidationFailed,
    InvalidManifest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    RateLimited,
    ServerError,
    NotImplemented,
    StorageError,
    TrustTransitionDenied,
    SignatureInvalid,
    HashMismatch,
    ArtifactNotAllowed,
    InsecureUrl,
    TooLarge,
    Timeout,
    /// Package version is yanked and no longer installable.
    PackageYanked,
    /// Attempt to modify an immutable package version.
    ImmutableVersion,
}

/// Public JSON error body.
#[derive(Debug, Serialize, Deserialize)]
pub struct PalaceError {
    pub code: PalaceErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl PalaceError {
    pub fn new(code: PalaceErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn status_code(&self) -> StatusCode {
        match self.code {
            PalaceErrorCode::BadRequest
            | PalaceErrorCode::ValidationFailed
            | PalaceErrorCode::InvalidManifest
            | PalaceErrorCode::TooLarge => StatusCode::BAD_REQUEST,
            PalaceErrorCode::Unauthorized => StatusCode::UNAUTHORIZED,
            PalaceErrorCode::Forbidden
            | PalaceErrorCode::TrustTransitionDenied
            | PalaceErrorCode::SignatureInvalid
            | PalaceErrorCode::HashMismatch
            | PalaceErrorCode::ArtifactNotAllowed
            | PalaceErrorCode::InsecureUrl => StatusCode::FORBIDDEN,
            PalaceErrorCode::NotFound
            | PalaceErrorCode::PackageYanked
            | PalaceErrorCode::ImmutableVersion => StatusCode::NOT_FOUND,
            PalaceErrorCode::Conflict => StatusCode::CONFLICT,
            PalaceErrorCode::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            PalaceErrorCode::NotImplemented => StatusCode::NOT_IMPLEMENTED,
            PalaceErrorCode::ServerError
            | PalaceErrorCode::StorageError
            | PalaceErrorCode::Timeout => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for PalaceError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = Json(self);
        (status, body).into_response()
    }
}

impl fmt::Display for PalaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{:?}] {}", self.code, self.message)
    }
}

impl std::error::Error for PalaceError {}

pub type PalaceResult<T> = Result<T, PalaceError>;

impl From<serde_json::Error> for PalaceError {
    fn from(e: serde_json::Error) -> Self {
        Self::new(
            PalaceErrorCode::BadRequest,
            format!("JSON parse error: {e}"),
        )
    }
}

impl From<uuid::Error> for PalaceError {
    fn from(e: uuid::Error) -> Self {
        Self::new(PalaceErrorCode::BadRequest, format!("UUID error: {e}"))
    }
}

impl From<axum::extract::rejection::JsonRejection> for PalaceError {
    fn from(e: axum::extract::rejection::JsonRejection) -> Self {
        Self::new(PalaceErrorCode::BadRequest, e.to_string())
    }
}
