//! Translate service failures to the application JSON error envelope.
use crate::utils::errors::{ErrorKind, ServiceError};
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
#[derive(Serialize)]
pub struct ErrorBody {
    pub error: String,
}
impl IntoResponse for ServiceError {
    fn into_response(self) -> Response {
        let status = match self.kind {
            ErrorKind::BadRequest => StatusCode::BAD_REQUEST,
            ErrorKind::NotFound => StatusCode::NOT_FOUND,
            ErrorKind::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (
            status,
            Json(ErrorBody {
                error: self.message,
            }),
        )
            .into_response()
    }
}

// Axum adds Allow after a method fallback. Unknown API responses must not carry it.
pub async fn method_headers(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let api = request.uri().path().starts_with("/api/");
    let mut response = next.run(request).await;
    if api && response.status() == StatusCode::NOT_FOUND {
        response.headers_mut().remove(axum::http::header::ALLOW);
    }
    response
}
