//! Shared validation for refs and repository-relative file paths.
use super::errors::ServiceError;
use std::path::{Component, Path};
pub fn valid_ref(value: &str) -> Result<(), ServiceError> {
    if value.is_empty() {
        Err(ServiceError::bad("empty ref"))
    } else if value.starts_with('-') || value.contains('\0') {
        Err(ServiceError::bad("invalid ref"))
    } else {
        Ok(())
    }
}

pub fn validate_path(value: &str) -> Result<(), ServiceError> {
    if value.is_empty() {
        return Err(ServiceError::bad("path is required"));
    }
    let mut parts = Vec::new();
    for part in Path::new(value).components() {
        match part {
            Component::Normal(p) => parts.push(p),
            Component::CurDir => {}
            Component::ParentDir if !parts.is_empty() => {
                parts.pop();
            }
            _ => return Err(ServiceError::bad("invalid path")),
        }
    }
    if parts
        .first()
        .is_some_and(|p| p.to_string_lossy().eq_ignore_ascii_case(".git"))
        || value.contains('\0')
    {
        return Err(ServiceError::bad("invalid path"));
    }
    Ok(())
}
