//! Fallback handlers only. Shared failure types live in utils/errors.rs.
use crate::utils::errors::ServiceError;
pub async fn not_found() -> ServiceError {
    ServiceError::missing("not found")
}
