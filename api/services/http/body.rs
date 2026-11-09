//! Client-compatible mutation JSON extraction with a bounded, single-document body.
use crate::utils::errors::ServiceError;
use axum::{
    body::Bytes,
    extract::{FromRequest, Request},
};
use serde::de::DeserializeOwned;
pub struct CompatJson<T>(pub T);
impl<T, S> FromRequest<S> for CompatJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ServiceError;
    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let bytes = Bytes::from_request(request, state)
            .await
            .map_err(|e| ServiceError::bad(e.to_string()))?;
        let mut value: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|e| ServiceError::bad(e.to_string()))?;
        // The API accepts case-insensitive field names and ignores null
        // for scalar values. Keep DTO defaults and optional patch fields intact.
        if let Some(object) = value.as_object_mut() {
            let fields = [
                "repo",
                "head",
                "base",
                "filePath",
                "startLine",
                "endLine",
                "type",
                "body",
                "author",
                "side",
                "summary",
                "resolved",
                "filePaths",
                "reviewed",
            ];
            let original = std::mem::take(object);
            for (key, value) in original {
                if value.is_null() {
                    continue;
                }
                let name = fields
                    .iter()
                    .find(|field| field.eq_ignore_ascii_case(&key))
                    .copied()
                    .unwrap_or(&key);
                object.insert(name.to_owned(), value);
            }
        }
        serde_json::from_value(value)
            .map(Self)
            .map_err(|e| ServiceError::bad(e.to_string()))
    }
}
