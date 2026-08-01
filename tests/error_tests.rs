use axum::http::StatusCode;
use k_o_palace::error::{PalaceError, PalaceErrorCode};

#[test]
fn immutable_versions_report_a_conflict() {
    let error = PalaceError::new(PalaceErrorCode::ImmutableVersion, "already published");
    assert_eq!(error.status_code(), StatusCode::CONFLICT);
}
