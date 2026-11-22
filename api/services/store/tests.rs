use super::*;
use crate::utils::errors::ErrorKind;
fn review(s: &Store) -> Review {
    s.create_or_get_review("/repo", "main", "feature", "sha1")
        .unwrap()
}
fn comment(s: &Store, id: i64, side: Side) -> Comment {
    s.add_comment(Comment {
        review_id: id,
        file_path: "a.rs".into(),
        start_line: 1,
        end_line: 2,
        comment_type: "bug".into(),
        body: "fix".into(),
        author: "human".into(),
        snippet: "old".into(),
        side,
        ..Default::default()
    })
    .unwrap()
}
fn mark(path: &str, hash: &str) -> FileReviewMark {
    FileReviewMark {
        path: path.into(),
        content_hash: hash.into(),
    }
}
#[test]
fn schema_foreign_keys_side_roundtrips_and_json_arrays() {
    let s = Store::in_memory().unwrap();
    let r = review(&s);
    let json = serde_json::to_value(&r).unwrap();
    assert_eq!(json["comments"], serde_json::json!([]));
    assert_eq!(json["reviewedFiles"], serde_json::json!([]));
    assert!(json.get("annotationError").is_none());
    for side in [Side::Head, Side::Index, Side::Worktree] {
        let c = comment(&s, r.id, side);
        assert_eq!(c.side, side);
        assert!(c.replies.is_empty());
        s.set_files_reviewed(r.id, &[mark("a.rs", "hash")], true, side)
            .unwrap();
        assert_eq!(s.list_reviewed_files_full(r.id).unwrap()[0].side, side);
    }
    assert_eq!(Side::from_flags(true, true), Side::Index);
    assert_eq!(Side::default(), Side::Head);
    assert_eq!(Side::parse(""), Some(Side::Head));
    for invalid in ["staged", "HEAD", "worktree "] {
        assert_eq!(Side::parse(invalid), None);
    }
    assert!(s.add_reply(999, "orphan", "agent").is_err());
    assert!(
        s.set_files_reviewed(999, &[mark("bad", "h")], true, Side::Head)
            .is_err()
    );
}
#[test]
fn review_resume_reset_and_missing_rows() {
    let s = Store::in_memory().unwrap();
    let r = review(&s);
    let c = comment(&s, r.id, Side::Head);
    s.add_reply(c.id, "reply", "agent").unwrap();
    s.set_files_reviewed(r.id, &[mark("a.rs", "h")], true, Side::Head)
        .unwrap();
    s.set_review_summary(r.id, "summary").unwrap();
    s.set_status(r.id, "exported").unwrap();
    let resumed = s
        .create_or_get_review("/repo", "main", "feature", "sha2")
        .unwrap();
    assert_eq!(resumed.id, r.id);
    assert_eq!(resumed.head_sha, "sha2");
    assert_eq!(resumed.status, "exported");
    assert_eq!(resumed.comments.len(), 1);
    assert_eq!(resumed.summary, "summary");
    assert_eq!(
        s.review_repo_head(r.id).unwrap(),
        ("/repo".into(), "feature".into())
    );
    s.reset_review(r.id).unwrap();
    let cleared = s.get_review(r.id).unwrap();
    assert!(cleared.comments.is_empty());
    assert!(cleared.reviewed_files.is_empty());
    assert!(cleared.summary.is_empty());
    assert_eq!(cleared.status, "exported");
    assert_eq!(s.get_comment(c.id).unwrap_err().kind, ErrorKind::NotFound);
    for err in [
        s.get_review(999).unwrap_err(),
        s.set_review_summary(999, "").unwrap_err(),
        s.delete_comment(999).unwrap_err(),
        s.delete_reply(999).unwrap_err(),
    ] {
        assert_eq!(err.kind, ErrorKind::NotFound);
    }
    // The API intentionally permits these no-op mutations for missing reviews.
    s.touch(999).unwrap();
    s.set_status(999, "draft").unwrap();
    s.reset_review(999).unwrap();
}
#[test]
fn comment_reply_mutations_timestamps_ordering_and_cascade() {
    let s = Store::in_memory().unwrap();
    let r = review(&s);
    let c = comment(&s, r.id, Side::Worktree);
    s.with(|db| {
        db.execute(
            "UPDATE comments SET updated_at='2001-02-03T04:05:06Z' WHERE id=?",
            [c.id],
        )?;
        Ok(())
    })
    .unwrap();
    assert_eq!(s.set_comment_resolved(c.id, true).unwrap(), r.id);
    assert_eq!(
        s.get_comment(c.id).unwrap().updated_at,
        "2001-02-03T04:05:06Z"
    );
    let c = s
        .update_comment(
            c.id,
            CommentEdit {
                body: "new body".into(),
                comment_type: "nit".into(),
                start: 5,
                end: 7,
                snippet: "new snippet".into(),
                commit_sha: "updated-sha".into(),
            },
        )
        .unwrap();
    assert_ne!(c.updated_at, "2001-02-03T04:05:06Z");
    assert_eq!(c.start_line, 5);
    assert_eq!(c.side, Side::Worktree);
    assert_eq!(c.snippet, "new snippet");
    assert!(c.resolved);
    let (a, rid) = s.add_reply(c.id, "first", "human").unwrap();
    assert_eq!(rid, r.id);
    let (b, _) = s.add_reply(c.id, "second", "agent").unwrap();
    let nested = s.get_review(r.id).unwrap();
    assert_eq!(
        nested.comments[0]
            .replies
            .iter()
            .map(|r| r.body.as_str())
            .collect::<Vec<_>>(),
        ["first", "second"]
    );
    s.with(|db| {
        db.execute(
            "UPDATE replies SET updated_at='2001-02-03T04:05:06Z' WHERE id=?",
            [a.id],
        )?;
        Ok(())
    })
    .unwrap();
    let (edited, rid) = s.update_reply(a.id, "edited").unwrap();
    assert_eq!(rid, r.id);
    assert_eq!(edited.author, "human");
    assert_ne!(edited.updated_at, "2001-02-03T04:05:06Z");
    assert_eq!(s.delete_reply(b.id).unwrap(), r.id);
    assert_eq!(s.get_comment(c.id).unwrap().replies.len(), 1);
    assert_eq!(s.delete_comment(c.id).unwrap(), r.id);
    assert_eq!(
        s.update_reply(a.id, "missing").unwrap_err().kind,
        ErrorKind::NotFound
    );
}
#[test]
fn reviewed_upsert_unmark_and_batch_rollback() {
    let s = Store::in_memory().unwrap();
    let r = review(&s);
    s.set_files_reviewed(r.id, &[mark("a", "h1"), mark("b", "hb")], true, Side::Index)
        .unwrap();
    s.set_files_reviewed(r.id, &[mark("a", "h2")], true, Side::Worktree)
        .unwrap();
    let full = s.list_reviewed_files_full(r.id).unwrap();
    assert_eq!(full.len(), 2);
    assert_eq!(full[0].content_hash, "h2");
    assert_eq!(full[0].side, Side::Worktree);
    s.set_files_reviewed(r.id, &[mark("a", "")], false, Side::Head)
        .unwrap();
    assert_eq!(s.get_review(r.id).unwrap().reviewed_files, ["b"]);
    s.with(|db| db.execute_batch("CREATE TRIGGER reject_mark BEFORE INSERT ON reviewed_files WHEN NEW.file_path='reject' BEGIN SELECT RAISE(ABORT,'reject'); END;")).unwrap();
    assert!(
        s.set_files_reviewed(
            r.id,
            &[mark("first", "h"), mark("reject", "h")],
            true,
            Side::Head
        )
        .is_err()
    );
    assert_eq!(s.get_review(r.id).unwrap().reviewed_files, ["b"]);
}
#[test]
fn pruning_preserves_exports_and_cascades_all_children() {
    let s = Store::in_memory().unwrap();
    let old = review(&s);
    let c = comment(&s, old.id, Side::Head);
    s.add_reply(c.id, "reply", "agent").unwrap();
    s.set_files_reviewed(old.id, &[mark("a", "h")], true, Side::Head)
        .unwrap();
    let exported = s
        .create_or_get_review("/repo", "main", "exported", "sha")
        .unwrap();
    s.set_status(exported.id, "exported").unwrap();
    let recent = s
        .create_or_get_review("/repo", "main", "recent", "sha")
        .unwrap();
    s.with(|db| {
        db.execute(
            "UPDATE reviews SET updated_at='2000-01-01T00:00:00Z' WHERE id IN (?,?)",
            params![old.id, exported.id],
        )?;
        Ok(())
    })
    .unwrap();
    assert_eq!(s.prune_drafts(Duration::from_secs(86400)).unwrap(), 1);
    assert!(s.get_review(exported.id).is_ok());
    assert!(s.get_review(recent.id).is_ok());
    assert_eq!(s.count_reviews().unwrap(), 2);
    s.with(|db| {
        for table in ["comments", "replies", "reviewed_files"] {
            let n: i64 =
                db.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))?;
            assert_eq!(n, 0);
        }
        Ok(())
    })
    .unwrap();
}
#[test]
fn concurrent_resume_is_one_row() {
    let s = Store::in_memory().unwrap();
    let handles: Vec<_> = (0..20)
        .map(|_| {
            let s = s.clone();
            std::thread::spawn(move || review(&s).id)
        })
        .collect();
    for h in handles {
        assert_eq!(h.join().unwrap(), 1);
    }
    assert_eq!(s.count_reviews().unwrap(), 1);
}
#[test]
fn migrates_regression_schema_preserving_data_and_is_idempotent() {
    let mut conn = Connection::open_in_memory().unwrap();
    // Baseline schema before additive columns were introduced.
    let mut sql = include_str!("schema.sql").to_string();
    let additions = [
        "summary",
        "resolved",
        "author",
        "commit_sha",
        "worktree",
        "indexed",
        "content_hash",
    ];
    sql = sql
        .lines()
        .filter(|line| {
            !additions
                .iter()
                .any(|name| line.trim_start().starts_with(&format!("{name} ")))
        })
        .collect::<Vec<_>>()
        .join("\n");
    conn.execute_batch(&sql).unwrap();
    conn.execute("INSERT INTO reviews (repo_path,base_ref,head_ref,head_sha,created_at,updated_at) VALUES ('/repo','main','legacy','sha','2000-01-01T00:00:00Z','2000-01-01T00:00:00Z')",[]).unwrap();
    conn.execute("INSERT INTO comments (review_id,file_path,start_line,end_line,body,created_at,updated_at) VALUES (1,'legacy.rs',1,1,'keep me','2000-01-01T00:00:00Z','2000-01-01T00:00:00Z')",[]).unwrap();
    conn.execute("INSERT INTO replies (comment_id,body,created_at,updated_at) VALUES (1,'keep reply','2000-01-01T00:00:00Z','2000-01-01T00:00:00Z')",[]).unwrap();
    conn.execute("INSERT INTO reviewed_files (review_id,file_path,reviewed_at) VALUES (1,'legacy.rs','2000-01-01T00:00:00Z')",[]).unwrap();
    schema::migrate(&mut conn).unwrap();
    schema::migrate(&mut conn).unwrap();
    let s = Store::initialize(conn).unwrap();
    let r = s.get_review(1).unwrap();
    assert_eq!(r.comments[0].body, "keep me");
    assert_eq!(r.comments[0].author, "reviewer");
    assert_eq!(r.comments[0].replies[0].author, "reviewer");
    assert_eq!(r.reviewed_files, ["legacy.rs"]);
    assert_eq!(r.comments[0].side, Side::Head);
    assert!(r.summary.is_empty());
    assert_eq!(r.scope_mode, "changes");
    s.with(|db| {
        db.execute("UPDATE comments SET worktree=1,indexed=1 WHERE id=1", [])?;
        Ok(())
    })
    .unwrap();
    assert_eq!(s.get_comment(1).unwrap().side, Side::Index);
}
#[test]
fn file_database_reopens_with_wal_and_stable_schema() {
    let dir = std::env::temp_dir().join(format!("reviewizard-sqlite-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("store.db");
    {
        let s = Store::open(&path).unwrap();
        let r = review(&s);
        comment(&s, r.id, Side::Index);
        let project = s.add_project("repo", "/repo", "/repo").unwrap();
        let codebase = s
            .save_scoped_review(SavedScope {
                mode: "codebase",
                project_id: project.id,
                path: "/repo",
                head: "main",
                base: "",
                sha: "sha1",
                from: "all",
                side: Side::Head,
            })
            .unwrap();
        assert_eq!(codebase.id, 2);
        s.with(|db| {
            let mode: String = db.pragma_query_value(None, "journal_mode", |r| r.get(0))?;
            assert_eq!(mode, "wal");
            Ok(())
        })
        .unwrap();
    }
    let s = Store::open(&path).unwrap();
    assert_eq!(s.get_review(1).unwrap().comments[0].side, Side::Index);
    assert_eq!(s.get_review(2).unwrap().scope_mode, "codebase");
    assert_eq!(s.get_review(1).unwrap().scope_mode, "changes");
    // An independent SQLite connection can read Rust-written schema/data without adaptation.
    let reader = Connection::open(&path).unwrap();
    assert_eq!(
        reader
            .query_row("SELECT body FROM comments", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "fix"
    );
    drop(reader);
    drop(s);
    std::fs::remove_dir_all(dir).unwrap();
}
#[tokio::test]
async fn async_adapter_runs_storage_on_blocking_workers() {
    let s = Store::in_memory().unwrap();
    let r = s
        .run(|s| s.create_or_get_review("/repo", "main", "async", "sha"))
        .await
        .unwrap();
    assert_eq!(r.head_ref, "async");
}

#[test]
fn failed_migration_rolls_back_schema_changes() {
    let mut db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE comments (id INTEGER PRIMARY KEY);")
        .unwrap();
    assert!(schema::migrate(&mut db).is_err());
    let exists: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name='reviews'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(exists, 0);
    let count: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('comments')",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}
#[test]
fn database_faults_and_constraint_failures_are_not_missing_rows() {
    let s = Store::in_memory().unwrap();
    assert_eq!(
        s.add_comment(Comment {
            review_id: 999,
            ..Default::default()
        })
        .unwrap_err()
        .kind,
        ErrorKind::Internal
    );
    s.with(|db| db.execute_batch("DROP TABLE reviews;"))
        .unwrap();
    assert_eq!(s.get_review(1).unwrap_err().kind, ErrorKind::Internal);
}

#[tokio::test]
async fn startup_retention_can_be_disabled_and_prunes_only_stale_drafts() {
    let store = Store::in_memory().unwrap();
    let r = review(&store);
    store
        .with(|db| {
            db.execute(
                "UPDATE reviews SET updated_at='2000-01-01T00:00:00Z' WHERE id=?",
                [r.id],
            )?;
            Ok(())
        })
        .unwrap();
    crate::services::application::apply_retention(&store, 0)
        .await
        .unwrap();
    crate::services::application::apply_retention(&store, -1)
        .await
        .unwrap();
    assert!(store.get_review(r.id).is_ok());
    crate::services::application::apply_retention(&store, 30)
        .await
        .unwrap();
    assert_eq!(
        store.get_review(r.id).unwrap_err().kind,
        ErrorKind::NotFound
    );
}

#[tokio::test]
async fn review_http_reads_report_database_fault_as_500() {
    use crate::services::{
        http::{self, AppState},
        workspace::Workspace,
    };
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let store = Store::in_memory().unwrap();
    review(&store);
    store
        .with(|db| db.execute_batch("DROP TABLE reviews"))
        .unwrap();
    let app = http::router(std::sync::Arc::new(AppState {
        workspace: Workspace::open(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap(),
        store,
        diffs: Default::default(),
        live: Default::default(),
    }));
    for (method, path) in [
        ("GET", "/api/reviews/1"),
        ("GET", "/api/reviews/1/comments"),
        ("POST", "/api/reviews/1/export"),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::INTERNAL_SERVER_ERROR,
            "{path}"
        );
    }
}

#[test]
fn deleting_review_cascades_and_preserves_other_reviews() {
    let s = Store::in_memory().unwrap();
    let r = review(&s);
    let c = comment(&s, r.id, Side::Head);
    let (reply, _) = s.add_reply(c.id, "reply", "human").unwrap();
    s.set_files_reviewed(r.id, &[mark("a.rs", "hash")], true, Side::Head)
        .unwrap();
    let other = s
        .create_or_get_review("/other", "main", "feature", "sha")
        .unwrap();
    s.delete_review(r.id).unwrap();
    assert!(matches!(
        s.get_review(r.id).unwrap_err().kind,
        ErrorKind::NotFound
    ));
    assert!(s.get_comment(c.id).is_err());
    assert_eq!(
        s.with(|c| c.query_row(
            "SELECT COUNT(*) FROM replies WHERE id=?",
            [reply.id],
            |row| row.get::<_, i64>(0)
        ))
        .unwrap(),
        0
    );
    assert!(s.list_reviewed_files_full(r.id).unwrap().is_empty());
    assert_eq!(s.get_review(other.id).unwrap().id, other.id);
    assert!(matches!(
        s.delete_review(r.id).unwrap_err().kind,
        ErrorKind::NotFound
    ));
}
