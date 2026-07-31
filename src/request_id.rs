//! Request ID middleware for tracing and error responses.

use axum::{extract::Request, http::HeaderName, middleware::Next, response::Response};
use uuid::Uuid;

/// Header name for request IDs.
pub const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

/// Extract or generate a request ID.
pub async fn request_id_middleware(mut req: Request, next: Next) -> Response {
    let request_id = req
        .headers()
        .get(&REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Add to request headers for downstream handlers
    req.headers_mut().insert(
        &REQUEST_ID_HEADER,
        request_id
            .parse()
            .unwrap_or_else(|_| "unknown".parse().unwrap()),
    );

    let mut response = next.run(req).await;

    // Add request ID to response headers
    response.headers_mut().insert(
        &REQUEST_ID_HEADER,
        request_id
            .parse()
            .unwrap_or_else(|_| "unknown".parse().unwrap()),
    );

    response
}
