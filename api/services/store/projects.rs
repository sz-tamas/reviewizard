use super::*;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub path: String,
    #[serde(skip)]
    pub canonical_path: String,
    pub archived: bool,
    pub review_count: i64,
    pub open_findings: i64,
    pub resolved_findings: i64,
    pub last_activity: String,
}

impl Store {
    pub fn validate_registered_repo(&self, path: &str) -> Result<()> {
        let canonical: Option<String> = self.with(|db| {
            db.query_row(
                "SELECT canonical_path FROM projects WHERE path=?1",
                [path],
                |r| r.get(0),
            )
            .optional()
        })?;
        if let Some(expected) = canonical {
            let actual = Path::new(path).canonicalize().map_err(ServiceError::from)?;
            if actual != Path::new(&expected) {
                return Err(ServiceError::bad("project folder identity has changed"));
            }
        }
        Ok(())
    }
    pub fn projects(&self) -> Result<Vec<Project>> {
        self.with(|db| {
            let mut q = db.prepare("SELECT p.id, p.name, p.path, p.canonical_path, p.archived,
                (SELECT COUNT(*) FROM reviews r WHERE r.repo_path = p.path),
                (SELECT COUNT(*) FROM comments c JOIN reviews r ON r.id=c.review_id WHERE r.repo_path=p.path AND c.resolved=0),
                (SELECT COUNT(*) FROM comments c JOIN reviews r ON r.id=c.review_id WHERE r.repo_path=p.path AND c.resolved<>0),
                COALESCE((SELECT MAX(updated_at) FROM reviews r WHERE r.repo_path=p.path), p.updated_at)
                FROM projects p ORDER BY p.id")?;
            q.query_map([], |r| Ok(Project {
                id: r.get(0)?, name: r.get(1)?, path: r.get(2)?, canonical_path: r.get(3)?,
                archived: r.get(4)?, review_count: r.get(5)?, open_findings: r.get(6)?,
                resolved_findings: r.get(7)?, last_activity: r.get(8)?,
            }))?.collect()
        })
    }

    pub fn add_project(&self, name: &str, path: &str, canonical: &str) -> Result<Project> {
        self.with(|db| {
            let tx = db.transaction()?;
            tx.execute("INSERT INTO projects(name,path,canonical_path,created_at,updated_at)
                VALUES(?1,?2,?3,?4,?4) ON CONFLICT(path) DO UPDATE SET archived=0, updated_at=excluded.updated_at
                WHERE projects.canonical_path=excluded.canonical_path", params![name,path,canonical,now()])?;
            tx.commit()
        })?;
        let project = self
            .projects()?
            .into_iter()
            .find(|p| p.path == path)
            .ok_or_else(|| ServiceError::internal("registered project missing"))?;
        if project.canonical_path != canonical {
            return Err(ServiceError::bad(
                "The folder now resolves to a different repository; its stored identity cannot be replaced",
            ));
        }
        Ok(project)
    }

    pub fn archive_project(&self, id: i64) -> Result<()> {
        let changed = self.with(|db| {
            db.execute(
                "UPDATE projects SET archived=1, updated_at=?2 WHERE id=?1",
                params![id, now()],
            )
        })?;
        if changed == 0 {
            return Err(ServiceError::missing("project not found"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn additive_project_migration_keeps_legacy_review_and_comment_ids() {
        let db = Connection::open_in_memory().unwrap();
        // Simulate the schema immediately before project registration was introduced.
        let old = include_str!("schema.sql")
            .split("CREATE TABLE IF NOT EXISTS projects")
            .next()
            .unwrap();
        db.execute_batch(old).unwrap();
        db.execute_batch("INSERT INTO reviews(id,repo_path,base_ref,head_ref,head_sha,created_at,updated_at)
            VALUES(77,'/legacy/repo','main','feature','sha','2026-01-01','2026-01-01');
            INSERT INTO comments(id,review_id,file_path,start_line,end_line,body,resolved,created_at,updated_at)
            VALUES(88,77,'a.txt',1,1,'open',0,'2026-01-01','2026-01-01'),
                  (89,77,'a.txt',2,2,'resolved',1,'2026-01-01','2026-01-01');").unwrap();
        let store = Store::initialize(db).unwrap();
        assert!(store.projects().unwrap().is_empty());
        let project = store
            .add_project("repo", "/legacy/repo", "/legacy/repo")
            .unwrap();
        assert_eq!(project.review_count, 1);
        assert_eq!(project.open_findings, 1);
        assert_eq!(project.resolved_findings, 1);
        let review = store.get_review(77).unwrap();
        assert_eq!(review.repo_path, "/legacy/repo");
        assert_eq!(
            review.comments.iter().map(|c| c.id).collect::<Vec<_>>(),
            [88, 89]
        );
        store.archive_project(project.id).unwrap();
        let restored = store
            .add_project("repo", "/legacy/repo", "/legacy/repo")
            .unwrap();
        assert_eq!(restored.id, project.id);
        assert!(!restored.archived);
        assert_eq!(store.get_review(77).unwrap().comments.len(), 2);
    }
}
