//! SQLite persistence. One mutex-protected connection serializes operations.
//! Call synchronous methods from blocking workers; run() is the HTTP-safe async adapter.
use crate::utils::errors::ServiceError;
use rusqlite::{Connection, OptionalExtension, params};
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};
mod comments;
pub mod models;
mod projects;
mod replies;
mod reviewed;
mod reviews;
mod schema;
mod scopes;
pub use models::*;
pub use projects::Project;
pub use scopes::SavedScope;

#[derive(Debug, Clone)]
pub struct Store {
    connection: Arc<Mutex<Connection>>,
}
pub type Result<T> = std::result::Result<T, ServiceError>;
fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}
impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        Self::initialize(Connection::open(path)?)
    }
    pub fn in_memory() -> Result<Self> {
        Self::initialize(Connection::open_in_memory()?)
    }
    pub fn open_directory(path: &Path) -> Result<Self> {
        std::fs::create_dir_all(path).map_err(ServiceError::internal)?;
        Self::open(&path.join("reviewizard.db"))
    }
    fn initialize(mut conn: Connection) -> Result<Self> {
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.pragma_update(None, "foreign_keys", true)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        schema::migrate(&mut conn)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(conn)),
        })
    }
    fn with<T>(&self, f: impl FnOnce(&mut Connection) -> rusqlite::Result<T>) -> Result<T> {
        let mut conn = self
            .connection
            .lock()
            .map_err(|_| ServiceError::internal("database lock poisoned"))?;
        f(&mut conn).map_err(ServiceError::from)
    }
    pub async fn run<T: Send + 'static>(
        &self,
        f: impl FnOnce(&Self) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || f(&store))
            .await
            .map_err(ServiceError::internal)?
    }
}
#[cfg(test)]
mod tests;
