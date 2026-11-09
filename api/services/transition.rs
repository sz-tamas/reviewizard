//! One-time, consistent backup before a pre-existing database is first opened by Rust.
//! No schema/data rewriting or automatic merging of independent review histories.
use crate::utils::errors::ServiceError;
use rusqlite::{Connection, OpenFlags};
use std::path::Path;
pub fn prepare(directory: &Path) -> Result<(), ServiceError> {
    std::fs::create_dir_all(directory).map_err(ServiceError::internal)?;
    let database = directory.join("reviewizard.db");
    let receipt = directory.join("rust-transition.json");
    if receipt.exists() {
        return Ok(());
    }
    let backup = directory.join("reviewizard.before-rust.db");
    if database.exists() {
        if backup.exists() {
            return Err(ServiceError::bad(
                "An incomplete Rust transition backup exists. Preserve it and select another --data-dir or inspect rust-transition.json before retrying.",
            ));
        }
        let connection = Connection::open_with_flags(&database, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        // VACUUM INTO takes a consistent SQLite snapshot, including committed WAL data.
        connection.execute("VACUUM INTO ?", [backup.to_string_lossy().as_ref()])?;
        let copy = Connection::open_with_flags(&backup, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let integrity: String = copy.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        if integrity != "ok" {
            return Err(ServiceError::internal(
                "transition backup failed integrity_check",
            ));
        }
        eprintln!("Initial database backup: {}", backup.display());
    }
    let record = serde_json::json!({"backend":"rust", "version":env!("REVIEWIZARD_VERSION"), "source":database, "backup":if backup.exists(){Some(backup)}else{None}, "createdAt":chrono::Utc::now().to_rfc3339(), "policy":"No automatic data merge; stop the server before rollback."});
    let temporary = directory.join("rust-transition.json.tmp");
    std::fs::write(&temporary, record.to_string()).map_err(ServiceError::internal)?;
    std::fs::rename(temporary, receipt).map_err(ServiceError::internal)?;
    Ok(())
}
