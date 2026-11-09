//! Framework-independent failures shared between services.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    BadRequest,
    NotFound,
    Internal,
}
#[derive(Debug)]
pub struct ServiceError {
    pub kind: ErrorKind,
    pub message: String,
}
impl ServiceError {
    pub fn bad(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::BadRequest,
            message: message.into(),
        }
    }
    pub fn missing(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::NotFound,
            message: message.into(),
        }
    }
    pub fn internal(message: impl ToString) -> Self {
        Self {
            kind: ErrorKind::Internal,
            message: message.to_string(),
        }
    }
}
impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for ServiceError {}
impl From<std::io::Error> for ServiceError {
    fn from(error: std::io::Error) -> Self {
        if error.kind() == std::io::ErrorKind::NotFound {
            Self::missing("not found")
        } else {
            Self::internal(error)
        }
    }
}

impl From<rusqlite::Error> for ServiceError {
    fn from(error: rusqlite::Error) -> Self {
        match error {
            rusqlite::Error::QueryReturnedNoRows => Self::missing("not found"),
            other => Self::internal(other),
        }
    }
}
