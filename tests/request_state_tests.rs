//! Regression tests for bounded request state and pagination arithmetic.

use k_o_palace::{pagination::Pagination, rate_limit::RateLimiter};

#[tokio::test]
async fn rate_limiter_keeps_independent_keys_independent() {
    let limiter = RateLimiter::new(1);

    limiter.check("publisher-a").await.unwrap();
    assert!(limiter.check("publisher-a").await.is_err());
    assert!(limiter.check("publisher-b").await.is_ok());
}

#[tokio::test]
async fn zero_request_limit_returns_a_rate_limit_error() {
    let limiter = RateLimiter::new(0);

    assert_eq!(limiter.check("anonymous").await, Err(60));
}

#[test]
fn bounds_saturate_at_usize_max() {
    let pagination = Pagination {
        limit: usize::MAX,
        offset: usize::MAX,
    };

    assert_eq!(pagination.bounds(usize::MAX), (usize::MAX, usize::MAX));
}

#[cfg(target_pointer_width = "64")]
#[test]
fn offset_rejects_values_that_would_not_fit_database_integer() {
    let result = Pagination::new(1, i64::MAX as usize + 1);

    assert!(result.is_err());
}
