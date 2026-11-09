//! Git edge cases and review-anchor regressions.
use super::*;
use reviewizard::{
    services::{
        git::{Repo, batch, refs},
        review,
        store::Side,
    },
    utils::errors::ErrorKind,
};

#[tokio::test]
async fn default_branch_fallback_chain_and_empty_branch_list() {
    let f = Fixture::new();
    let repo = Repo { path: f.0.clone() };
    assert_eq!(repo.main_branch().await, "main");
    f.git(&["branch", "-m", "main", "master"]);
    assert_eq!(repo.main_branch().await, "master");
    f.git(&["branch", "-D", "master"]);
    assert_eq!(repo.main_branch().await, "");
    f.git(&["update-ref", "refs/remotes/origin/develop", "HEAD"]);
    f.git(&[
        "symbolic-ref",
        "refs/remotes/origin/HEAD",
        "refs/remotes/origin/develop",
    ]);
    assert_eq!(repo.main_branch().await, "origin/develop");
    f.git(&["symbolic-ref", "--delete", "refs/remotes/origin/HEAD"]);
    f.git(&["update-ref", "-d", "refs/remotes/origin/develop"]);
    f.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    assert_eq!(repo.main_branch().await, "origin/main");
    f.git(&["update-ref", "-d", "refs/remotes/origin/main"]);
    f.git(&["update-ref", "refs/remotes/origin/master", "HEAD"]);
    assert_eq!(repo.main_branch().await, "origin/master");
    assert_eq!(
        refs::parent(&repo, "main").await.unwrap_err().kind,
        ErrorKind::Internal
    );
    f.git(&["update-ref", "-d", "refs/remotes/origin/master"]);
    f.git(&["update-ref", "-d", "refs/heads/feature/test"]);
    let list = f.json(&format!("/api/branches?repo={}", f.repo())).await;
    assert_eq!(list["branches"], serde_json::json!([]));
}
#[tokio::test]
async fn batch_objects_index_binary_newlines_empty_and_absence() {
    let f = Fixture::new();
    for (path, bytes) in [
        ("empty", b"".as_slice()),
        ("space file", b"a\nb".as_slice()),
        ("tricky", b"header blob 2\n\0z\n".as_slice()),
    ] {
        std::fs::write(f.0.join(path), bytes).unwrap();
    }
    f.commit();
    let repo = Repo { path: f.0.clone() };
    let specs = vec![
        "HEAD:empty".into(),
        "HEAD:space file".into(),
        "HEAD:tricky".into(),
        "HEAD:gone".into(),
        "HEAD:space file".into(),
        "HEAD:bad\nname".into(),
    ];
    let objects = batch::objects(&repo, &specs).await.unwrap();
    assert!(objects["HEAD:empty"].is_empty());
    assert_eq!(objects["HEAD:space file"], b"a\nb");
    assert_eq!(objects["HEAD:tricky"], b"header blob 2\n\0z\n");
    assert!(!objects.contains_key("HEAD:gone"));
    assert!(!objects.contains_key("HEAD:bad\nname"));
    std::fs::write(f.0.join("space file"), "staged").unwrap();
    f.git(&["add", "space file"]);
    let objects = batch::objects(&repo, &[":space file".into()])
        .await
        .unwrap();
    assert_eq!(objects[":space file"], b"staged");
    assert!(batch::objects(&repo, &[]).await.unwrap().is_empty());
    f.git(&["mv", "space file", "moved"]);
    f.commit();
    for spec in ["HEAD:space file", ":space file", "absent:moved"] {
        assert_eq!(
            repo.object(spec).await.unwrap_err().kind,
            ErrorKind::NotFound
        );
    }
    assert_eq!(
        repo.worktree_file("space file").await.unwrap_err().kind,
        ErrorKind::NotFound
    );
}
#[tokio::test]
async fn rename_with_shift_edited_rename_and_replaced_path() {
    let f = Fixture::new();
    let repo = Repo { path: f.0.clone() };
    std::fs::write(f.0.join("old.txt"), "l1\nl2\nl3\nl4\nl5\n").unwrap();
    let old = f.commit();
    f.git(&["mv", "old.txt", "new.txt"]);
    std::fs::write(f.0.join("new.txt"), "l0\nl1\nl2\nl3\nl4\nl5\n").unwrap();
    f.commit();
    let mut c = anchor("old.txt", 2, 3, "l2\nl3", &old, Side::Head);
    review::annotate_comment(Some(&repo), "HEAD", "", &mut c).await;
    assert_eq!(c.anchor_status, "moved");
    assert_eq!(c.current_file_path, "new.txt");
    assert_eq!((c.current_start_line, c.current_end_line), (3, 4));
    std::fs::write(f.0.join("new.txt"), "l0\nl1\nl2\nEDIT\nl4\nl5\n").unwrap();
    f.commit();
    review::annotate_comment(Some(&repo), "HEAD", "", &mut c).await;
    assert_eq!(c.anchor_status, "outdated");
    assert!(c.current_file_path.is_empty());
    std::fs::write(f.0.join("a.go"), "aaa\nbbb\nccc\n").unwrap();
    std::fs::write(f.0.join("settings.go"), "sx\nsy\nsz\n").unwrap();
    let old = f.commit();
    std::fs::write(f.0.join("a.go"), "sx\nsy\nsz\n").unwrap();
    std::fs::remove_file(f.0.join("settings.go")).unwrap();
    f.commit();
    let mut c = anchor("a.go", 2, 2, "bbb", &old, Side::Head);
    review::annotate_comment(Some(&repo), "HEAD", "", &mut c).await;
    assert_eq!(c.anchor_status, "outdated");
    assert!(c.current_file_path.is_empty());
}
#[tokio::test]
async fn missing_file_no_sha_and_empty_snippet_annotations() {
    let f = Fixture::new();
    let repo = Repo { path: f.0.clone() };
    for (path, start, snip, want) in [
        ("a.txt", 1, "committed", "current"),
        ("gone", 1, "missing", "outdated"),
        ("gone", 0, "", "current"),
        ("gone", 1, " \n", "current"),
    ] {
        let mut c = anchor(path, start, start, snip, "", Side::Head);
        review::annotate_comment(Some(&repo), "HEAD", "", &mut c).await;
        assert_eq!(c.anchor_status, want);
    }
}
#[tokio::test]
async fn fingerprints_stability_deletion_branching_and_returned_deletion() {
    use reviewizard::services::git::fingerprint;
    let f = Fixture::new();
    f.commit();
    let repo = Repo { path: f.0.clone() };
    let tree = fingerprint::worktree(&repo).await.unwrap();
    let refs = fingerprint::refs(&repo).await.unwrap();
    std::fs::write(f.0.join("a.txt"), "working\n").unwrap();
    assert_eq!(tree, fingerprint::worktree(&repo).await.unwrap());
    std::fs::remove_file(f.0.join("a.txt")).unwrap();
    assert_ne!(tree, fingerprint::worktree(&repo).await.unwrap());
    f.git(&["branch", "another"]);
    assert_ne!(refs, fingerprint::refs(&repo).await.unwrap());
    let hash = review::fingerprint_files(&repo, "HEAD", &["missing".into()], Side::Worktree).await;
    assert_eq!(hash["missing"], "absent");
}
#[tokio::test]
async fn repository_and_head_loss_preserve_marks_and_heal() {
    use reviewizard::services::store::{Review, ReviewedFile};
    let f = Fixture::new();
    let repo = Repo { path: f.0.clone() };
    let hashes = review::fingerprint_files(&repo, "HEAD", &["a.txt".into()], Side::Head).await;
    let marks = vec![ReviewedFile {
        path: "a.txt".into(),
        content_hash: hashes["a.txt"].clone(),
        side: Side::Head,
    }];
    let baseline = Review {
        repo_path: f.0.to_string_lossy().into(),
        head_ref: "feature/test".into(),
        reviewed_files: vec!["a.txt".into()],
        comments: vec![anchor("a.txt", 1, 1, "committed", "", Side::Head)],
        ..Default::default()
    };
    let moved = f.0.with_extension("moved");
    std::fs::rename(&f.0, &moved).unwrap();
    let mut r = baseline.clone();
    review::annotate(&mut r, Some(&marks), None).await;
    std::fs::rename(&moved, &f.0).unwrap();
    assert!(!r.annotation_error.is_empty());
    assert_eq!(r.reviewed_files, ["a.txt"]);
    assert_ne!(r.comments[0].anchor_status, "outdated");
    f.git(&["switch", "main"]);
    f.git(&["branch", "-D", "feature/test"]);
    let mut r = baseline.clone();
    review::annotate(&mut r, Some(&marks), None).await;
    assert!(!r.annotation_error.is_empty());
    assert_eq!(r.reviewed_files, ["a.txt"]);
    f.git(&["branch", "feature/test"]);
    let mut r = baseline;
    review::annotate(&mut r, Some(&marks), None).await;
    assert!(r.annotation_error.is_empty());
    assert_eq!(r.comments[0].anchor_status, "current");
}
#[tokio::test]
async fn missing_review_reads_all_report_404() {
    for (method, path) in [
        ("GET", "/api/reviews/999"),
        ("GET", "/api/reviews/999/comments"),
        ("POST", "/api/reviews/999/export"),
        ("POST", "/api/reviews/999/export.md"),
    ] {
        let response = app()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
async fn zero_ids_and_duplicate_query_keys_preserve_wire_contract() {
    let f = Fixture::new();
    let response = f.get("/api/reviews/0").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let first = f
        .json(&format!("/api/diff?repo={}&head=HEAD&base=main", f.repo()))
        .await;
    let duplicate = f
        .json(&format!(
            "/api/diff?repo={}&head=HEAD&base=main&%62ase=stale",
            f.repo()
        ))
        .await;
    assert_eq!(first, duplicate);
}
#[cfg(unix)]
#[tokio::test]
async fn corrupt_git_blob_is_500_not_a_worktree_fallback() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(&f.0)
        .args(["rev-parse", "HEAD:a.txt"])
        .output()
        .unwrap();
    let oid = String::from_utf8(output.stdout).unwrap();
    let oid = oid.trim();
    let object = f.0.join(".git/objects").join(&oid[..2]).join(&oid[2..]);
    std::fs::set_permissions(&object, std::fs::Permissions::from_mode(0o644)).unwrap();
    std::fs::write(&object, "not a valid object").unwrap();
    let response = f
        .get(&format!("/api/file?repo={}&ref=HEAD&path=a.txt", f.repo()))
        .await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
#[cfg(unix)]
#[tokio::test]
async fn in_root_symlink_is_readable_and_git_directory_alias_is_rejected() {
    let f = Fixture::new();
    let repo = Repo { path: f.0.clone() };
    std::os::unix::fs::symlink(f.0.join("a.txt"), f.0.join("inside")).unwrap();
    assert_eq!(repo.worktree_file("inside").await.unwrap(), b"working\n");
    std::os::unix::fs::symlink(f.0.join(".git"), f.0.join("gitlink")).unwrap();
    assert!(repo.worktree_file("gitlink/config").await.is_err());
}
