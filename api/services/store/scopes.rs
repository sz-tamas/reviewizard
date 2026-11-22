use super::*;

pub struct SavedScope<'a> {
    pub mode: &'a str,
    pub project_id: i64,
    pub path: &'a str,
    pub head: &'a str,
    pub base: &'a str,
    pub sha: &'a str,
    pub from: &'a str,
    pub side: Side,
}

impl Store {
    pub fn save_scoped_review(&self, scope: SavedScope<'_>) -> Result<Review> {
        self.with(|db| {
            let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let side = match scope.side { Side::Head => "head", Side::Index => "index", Side::Worktree => "worktree" };
            tx.execute("INSERT INTO reviews(repo_path,head_ref,base_ref,head_sha,project_id,scope_from,scope_side,scope_configured,scope_mode,created_at,updated_at)
                VALUES(?1,?2,?3,?4,?5,?6,?7,1,?8,?9,?9)", params![scope.path,scope.head,scope.base,scope.sha,scope.project_id,scope.from,side,scope.mode,now()])?;
            let id = tx.last_insert_rowid();
            tx.commit()?;
            super::reviews::get(db, id)
        })
    }

    pub fn project_reviews(&self, project_id: i64) -> Result<Vec<Review>> {
        self.with(|db| {
            let path: String =
                db.query_row("SELECT path FROM projects WHERE id=?1", [project_id], |r| {
                    r.get(0)
                })?;
            let ids = db
                .prepare("SELECT id FROM reviews WHERE repo_path=?1 ORDER BY id DESC")?
                .query_map([path], |r| r.get::<_, i64>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            ids.into_iter()
                .map(|id| super::reviews::get(db, id))
                .collect()
        })
    }

    pub fn set_review_side(&self, id: i64, side: Side) -> Result<()> {
        let side = match side {
            Side::Head => "head",
            Side::Index => "index",
            Side::Worktree => "worktree",
        };
        self.with(|db| {
            if db.execute(
                "UPDATE reviews SET scope_side=?2,scope_configured=1,updated_at=?3 WHERE id=?1",
                params![id, side, now()],
            )? == 0
            {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            Ok(())
        })
    }
    pub fn set_review_status(&self, id: i64, status: &str) -> Result<()> {
        self.with(|db| {
            if db.execute(
                "UPDATE reviews SET status=?2,updated_at=?3 WHERE id=?1",
                params![id, status, now()],
            )? == 0
            {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_assignments_keep_their_scope_and_survive_legacy_draft_pruning() {
        let store = Store::in_memory().unwrap();
        let project = store.add_project("repo", "/repo", "/repo").unwrap();
        let saved = store
            .save_scoped_review(SavedScope {
                mode: "changes",
                project_id: project.id,
                path: "/repo",
                head: "feature",
                base: "main",
                sha: "head-sha",
                from: "from-sha",
                side: Side::Index,
            })
            .unwrap();
        let legacy = store
            .create_or_get_review("/repo", "main", "feature", "sha")
            .unwrap();
        assert_ne!(saved.id, legacy.id);
        store
            .with(|db| db.execute("UPDATE reviews SET updated_at='2000-01-01T00:00:00Z'", []))
            .unwrap();
        assert_eq!(store.prune_drafts(Duration::from_secs(1)).unwrap(), 1);
        assert!(store.get_review(legacy.id).is_err());
        let reopened = store.get_review(saved.id).unwrap();
        assert_eq!(reopened.scope_side, Side::Index);
        assert_eq!(reopened.scope_from, "from-sha");
        assert_eq!(reopened.project_id, Some(project.id));
        assert!(reopened.scope_configured);
    }
}
