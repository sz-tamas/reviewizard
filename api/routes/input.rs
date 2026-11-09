//! HTTP extraction errors use the same JSON envelope as service failures.
use crate::utils::errors::ServiceError;
use axum::extract::{Path, rejection::PathRejection};
pub fn id(path: Result<Path<i64>, PathRejection>) -> Result<i64, ServiceError> {
    let Path(id) = path.map_err(|e| ServiceError::bad(e.body_text()))?;
    Ok(id)
}
pub fn body<T>(
    input: Result<crate::services::http::body::CompatJson<T>, ServiceError>,
) -> Result<T, ServiceError> {
    input.map(|crate::services::http::body::CompatJson(v)| v)
}
