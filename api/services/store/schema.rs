use rusqlite::Connection;
pub fn migrate(conn: &mut Connection) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    tx.execute_batch(include_str!("schema.sql"))?;
    let additions = [
        (
            "reviews",
            "scope_mode",
            "scope_mode TEXT NOT NULL DEFAULT 'changes'",
        ),
        (
            "reviews",
            "project_id",
            "project_id INTEGER REFERENCES projects(id)",
        ),
        (
            "reviews",
            "scope_from",
            "scope_from TEXT NOT NULL DEFAULT 'all'",
        ),
        (
            "reviews",
            "scope_side",
            "scope_side TEXT NOT NULL DEFAULT 'head'",
        ),
        (
            "reviews",
            "scope_configured",
            "scope_configured INTEGER NOT NULL DEFAULT 0",
        ),
        ("reviews", "summary", "summary TEXT NOT NULL DEFAULT ''"),
        (
            "comments",
            "resolved",
            "resolved INTEGER NOT NULL DEFAULT 0",
        ),
        (
            "comments",
            "author",
            "author TEXT NOT NULL DEFAULT 'reviewer'",
        ),
        (
            "comments",
            "worktree",
            "worktree INTEGER NOT NULL DEFAULT 0",
        ),
        (
            "comments",
            "commit_sha",
            "commit_sha TEXT NOT NULL DEFAULT ''",
        ),
        ("comments", "indexed", "indexed INTEGER NOT NULL DEFAULT 0"),
        (
            "reviewed_files",
            "content_hash",
            "content_hash TEXT NOT NULL DEFAULT ''",
        ),
        (
            "reviewed_files",
            "worktree",
            "worktree INTEGER NOT NULL DEFAULT 0",
        ),
        (
            "reviewed_files",
            "indexed",
            "indexed INTEGER NOT NULL DEFAULT 0",
        ),
        (
            "replies",
            "author",
            "author TEXT NOT NULL DEFAULT 'reviewer'",
        ),
    ];
    for (table, column, ddl) in additions {
        let exists = {
            let mut query = tx.prepare(&format!("PRAGMA table_info({table})"))?;
            let names = query
                .query_map([], |r| r.get::<_, String>(1))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            names.iter().any(|name| name == column)
        };
        if !exists {
            tx.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {ddl}"))?;
        }
    }
    tx.commit()
}
