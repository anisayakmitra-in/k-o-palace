//! Security primitives: hashing, token handling, and HMAC helpers.

use crate::error::{PalaceError, PalaceErrorCode, PalaceResult};
use sha2::{Digest, Sha256};

/// Compute a SHA-256 hex digest of bytes.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Verify a SHA-256 content hash.
///
/// `claimed` should be `sha256:<hex>`.
pub fn verify_content_hash(content: &[u8], claimed: Option<&str>) -> PalaceResult<()> {
    let claimed = claimed
        .ok_or_else(|| PalaceError::new(PalaceErrorCode::HashMismatch, "missing content_hash"))?;
    let expected = claimed.strip_prefix("sha256:").ok_or_else(|| {
        PalaceError::new(
            PalaceErrorCode::HashMismatch,
            "content_hash must start with 'sha256:'",
        )
    })?;
    let actual = sha256_hex(content);
    if actual != expected {
        return Err(PalaceError::new(
            PalaceErrorCode::HashMismatch,
            "content hash mismatch",
        ));
    }
    Ok(())
}

/// Redact a token for logging.
pub fn redact_token(token: &str) -> String {
    if token.len() <= 8 {
        "***".to_string()
    } else {
        format!("{}...{}", &token[..4], &token[token.len() - 4..])
    }
}
