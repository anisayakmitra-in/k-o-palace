//! Trust levels, transitions, and signature verification.

use crate::auth::AuthContext;
use crate::error::{PalaceError, PalaceErrorCode, PalaceResult};
use crate::models::{TrustInfo, TrustLevel, TrustTransition};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::Utc;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use uuid::Uuid;

/// Allowed trust transitions. Each key lists reachable next levels.
const TRUST_GRAPH: &[(&str, &[&str])] = &[
    (
        "experimental",
        &[
            "community",
            "verified",
            "official",
            "enterprise",
            "certified",
        ],
    ),
    (
        "community",
        &["verified", "official", "enterprise", "certified"],
    ),
    ("verified", &["official", "enterprise", "certified"]),
    ("official", &["enterprise", "certified"]),
    ("enterprise", &["certified"]),
    ("certified", &[]),
];

/// Verify an Ed25519 signature for a package.
pub fn verify_signature(trust: &TrustInfo, content: &[u8]) -> PalaceResult<()> {
    let signature_b64 = trust
        .signature
        .as_deref()
        .ok_or_else(|| PalaceError::new(PalaceErrorCode::SignatureInvalid, "missing signature"))?;
    let public_key_b64 = trust
        .public_key
        .as_deref()
        .ok_or_else(|| PalaceError::new(PalaceErrorCode::SignatureInvalid, "missing public_key"))?;

    let signature_bytes = BASE64.decode(signature_b64).map_err(|_| {
        PalaceError::new(
            PalaceErrorCode::SignatureInvalid,
            "invalid base64 signature",
        )
    })?;
    let public_key_bytes = BASE64.decode(public_key_b64).map_err(|_| {
        PalaceError::new(
            PalaceErrorCode::SignatureInvalid,
            "invalid base64 public_key",
        )
    })?;

    let signature = Signature::from_slice(&signature_bytes).map_err(|_| {
        PalaceError::new(PalaceErrorCode::SignatureInvalid, "invalid signature bytes")
    })?;
    let public_key =
        VerifyingKey::from_bytes(public_key_bytes.as_slice().try_into().map_err(|_| {
            PalaceError::new(
                PalaceErrorCode::SignatureInvalid,
                "invalid public key length",
            )
        })?)
        .map_err(|_| {
            PalaceError::new(PalaceErrorCode::SignatureInvalid, "invalid verifying key")
        })?;

    public_key.verify(content, &signature).map_err(|_| {
        PalaceError::new(
            PalaceErrorCode::SignatureInvalid,
            "signature verification failed",
        )
    })?;

    Ok(())
}

/// Verify the package content hash.
pub fn verify_content_hash(content: &[u8], trust: &TrustInfo) -> PalaceResult<()> {
    crate::security::verify_content_hash(content, trust.content_hash.as_deref())
}

/// Determine whether a trust transition is allowed.
pub fn can_transition(from: TrustLevel, to: TrustLevel) -> bool {
    if from == to {
        return true;
    }
    let from_str = from.as_str();
    let to_str = to.as_str();
    TRUST_GRAPH
        .iter()
        .find(|(s, _)| *s == from_str)
        .map(|(_, targets)| targets.contains(&to_str))
        .unwrap_or(false)
}

/// Apply a trust transition, enforcing authorization and recording audit.
pub async fn transition_trust(
    repo: &crate::repository::PackageRepository,
    actor: &AuthContext,
    package_id: &str,
    new_level: TrustLevel,
    reason: Option<String>,
) -> PalaceResult<TrustTransition> {
    if !actor.can_moderate() && !actor.can_administer() {
        return Err(PalaceError::new(
            PalaceErrorCode::Forbidden,
            "insufficient role to change trust level",
        ));
    }

    // Server-assigned levels require moderator+; clients can't forge them.
    if !actor.can_moderate() && new_level.is_server_assigned() {
        return Err(PalaceError::new(
            PalaceErrorCode::TrustTransitionDenied,
            "only moderators can assign server trust levels",
        ));
    }

    let package = repo.get_package(package_id).await?;
    let current = package.trust.level;

    if new_level.is_server_assigned() {
        if let Some(publisher_id) = repo.get_package_publisher_id(package_id).await? {
            let verification = repo.get_publisher_verification(publisher_id).await?;
            if !verification.verified {
                return Err(PalaceError::new(
                    PalaceErrorCode::TrustTransitionDenied,
                    "publisher verification is required for server-assigned trust levels",
                ));
            }
        }
    }
    if !can_transition(current.clone(), new_level.clone()) {
        return Err(PalaceError::new(
            PalaceErrorCode::TrustTransitionDenied,
            format!(
                "trust transition from {} to {} is not allowed",
                current.as_str(),
                new_level.as_str()
            ),
        ));
    }

    let transition = TrustTransition {
        id: Uuid::new_v4(),
        package_id: package_id.into(),
        from_level: current.as_str().to_string(),
        to_level: new_level.as_str().to_string(),
        approved_by: actor.publisher.id,
        reason,
        created_at: Utc::now(),
    };

    repo.record_trust_transition(&transition).await?;
    Ok(transition)
}
