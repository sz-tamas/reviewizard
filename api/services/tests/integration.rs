use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use reviewizard::services::{
    http::{self, AppState},
    store::Store,
    workspace::Workspace,
};
use std::sync::Arc;
use tower::ServiceExt;
fn app() -> axum::Router {
    http::router(Arc::new(AppState {
        workspace: Workspace::open(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap(),
        store: Store::in_memory().unwrap(),
        diffs: Default::default(),
        live: Default::default(),
    }))
}
#[tokio::test]
async fn health_and_missing_reviews_have_distinct_status_codes() {
    for (path, expected) in [
        ("/healthz", StatusCode::OK),
        ("/api/reviews/1", StatusCode::NOT_FOUND),
        ("/api/missing", StatusCode::NOT_FOUND),
    ] {
        let response = app()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
}
#[tokio::test]
async fn browser_cross_site_writes_are_refused_but_local_agents_are_allowed() {
    for (site, expected) in [
        (Some("cross-site"), StatusCode::FORBIDDEN),
        (Some("same-origin"), StatusCode::BAD_REQUEST),
        (None, StatusCode::BAD_REQUEST),
    ] {
        let mut request = Request::builder().method("POST").uri("/api/reviews");
        if let Some(site) = site {
            request = request.header("sec-fetch-site", site);
        }
        let response = app()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
}

struct Fixture(std::path::PathBuf);
impl Fixture {
    fn new() -> Self {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "reviewizard-routes-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        let f = Self(path);
        f.git(&["init", "-b", "main"]);
        f.git(&["config", "user.name", "Fixture"]);
        f.git(&["config", "user.email", "fixture@example.invalid"]);
        std::fs::write(f.0.join("a.txt"), "committed\n").unwrap();
        std::fs::write(f.0.join("image.png"), [0x89, b'P', b'N', b'G', 0, 0xff]).unwrap();
        f.git(&["add", "."]);
        f.git(&["commit", "-m", "initial"]);
        f.git(&["switch", "-c", "feature/test"]);
        std::fs::write(f.0.join("feature.txt"), "feature\n").unwrap();
        f.git(&["add", "."]);
        f.git(&["commit", "-m", "feature"]);
        std::fs::write(f.0.join("a.txt"), "staged\n").unwrap();
        f.git(&["add", "a.txt"]);
        std::fs::write(f.0.join("a.txt"), "working\n").unwrap();
        std::fs::write(f.0.join("untracked.txt"), "new\n").unwrap();
        f
    }
    fn git(&self, args: &[&str]) {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&self.0)
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env(
                "GIT_CONFIG_GLOBAL",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
            )
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    fn app(&self) -> axum::Router {
        http::router(Arc::new(AppState {
            workspace: Workspace::open(&self.0).unwrap(),
            store: Store::in_memory().unwrap(),
            diffs: Default::default(),
            live: Default::default(),
        }))
    }
    fn repo(&self) -> &str {
        self.0.file_name().unwrap().to_str().unwrap()
    }
    async fn get(&self, endpoint: &str) -> axum::response::Response {
        self.app()
            .oneshot(
                Request::builder()
                    .uri(endpoint)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
    }
    async fn json(&self, endpoint: &str) -> serde_json::Value {
        let response = self.get(endpoint).await;
        assert_eq!(response.status(), StatusCode::OK, "{endpoint}");
        let bytes = axum::body::to_bytes(response.into_body(), 8 << 20)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[tokio::test]
async fn repository_branches_files_and_commits_have_response_shapes() {
    let f = Fixture::new();
    let repos = f.json("/api/repos").await;
    assert_eq!(repos["repos"][0]["name"], f.repo());
    assert_eq!(
        repos["repos"][0]["lastActivity"].as_str().unwrap().len(),
        10
    );
    let branches = f.json(&format!("/api/branches?repo={}", f.repo())).await;
    assert_eq!(branches["branches"][0]["name"], "main");
    assert_eq!(branches["branches"][0]["isMain"], true);
    assert_eq!(branches["branches"][1]["name"], "feature/test");
    assert_eq!(branches["branches"][1]["isCurrent"], true);
    let files = f
        .json(&format!("/api/files?repo={}&ref=HEAD", f.repo()))
        .await;
    assert_eq!(
        files["files"],
        serde_json::json!(["a.txt", "feature.txt", "image.png"])
    );
    let commits = f
        .json(&format!(
            "/api/commits?repo={}&ref=HEAD&base=main",
            f.repo()
        ))
        .await;
    assert_eq!(commits["commits"].as_array().unwrap().len(), 1);
    assert_eq!(commits["commits"][0]["subject"], "feature");
    assert!(commits["commits"][0]["shortSha"].is_string());
    let empty = f
        .json(&format!(
            "/api/commits?repo={}&ref=main&base=main",
            f.repo()
        ))
        .await;
    assert!(empty["commits"].is_null()); // Empty history is represented as null.
}

#[tokio::test]
async fn file_reads_distinguish_committed_staged_worktree_and_fallback() {
    let f = Fixture::new();
    for (side, content, worktree) in [
        ("head", "committed\n", false),
        ("index", "staged\n", false),
        ("worktree", "working\n", true),
    ] {
        let file = f
            .json(&format!(
                "/api/file?repo={}&ref=HEAD&path=a.txt&side={side}",
                f.repo()
            ))
            .await;
        assert_eq!(file["content"], content);
        assert_eq!(file["worktree"], worktree);
    }
    let file = f
        .json(&format!(
            "/api/file?repo={}&ref=HEAD&path=untracked.txt",
            f.repo()
        ))
        .await;
    assert_eq!(file["content"], "new\n");
    assert_eq!(file["worktree"], true);
    for (suffix, status) in [
        ("ref=HEAD&path=missing", StatusCode::NOT_FOUND),
        ("ref=-x&path=a.txt", StatusCode::BAD_REQUEST),
        ("ref=HEAD&path=a.txt&side=wrong", StatusCode::BAD_REQUEST),
        (
            "ref=HEAD&path=../outside&side=worktree",
            StatusCode::BAD_REQUEST,
        ),
        (
            "ref=HEAD&path=.git/config&side=worktree",
            StatusCode::BAD_REQUEST,
        ),
    ] {
        let response = f
            .get(&format!("/api/file?repo={}&{suffix}", f.repo()))
            .await;
        assert_eq!(response.status(), status, "{suffix}");
        let bytes = axum::body::to_bytes(response.into_body(), 8 << 20)
            .await
            .unwrap();
        assert!(serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["error"].is_string());
    }
    assert!(!f.0.join("unused-data").exists());
}

#[tokio::test]
async fn blobs_preserve_bytes_and_security_headers() {
    let f = Fixture::new();
    let response = f
        .get(&format!(
            "/api/blob?repo={}&ref=HEAD&path=image.png",
            f.repo()
        ))
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "image/png");
    assert_eq!(response.headers()["x-content-type-options"], "nosniff");
    assert!(
        response.headers()["content-security-policy"]
            .to_str()
            .unwrap()
            .contains("sandbox")
    );
    let bytes = axum::body::to_bytes(response.into_body(), 100)
        .await
        .unwrap();
    assert_eq!(bytes.as_ref(), &[0x89, b'P', b'N', b'G', 0, 0xff]);
}

#[cfg(unix)]
#[tokio::test]
async fn workspace_and_worktree_symlinks_cannot_escape_roots_or_read_git_config() {
    let f = Fixture::new();
    let outside = Fixture::new();
    std::os::unix::fs::symlink(outside.0.join("a.txt"), f.0.join("escape")).unwrap();
    std::os::unix::fs::symlink(f.0.join(".git/config"), f.0.join("secret")).unwrap();
    for path in ["escape", "secret"] {
        assert_eq!(
            f.get(&format!(
                "/api/file?repo={}&path={path}&side=worktree",
                f.repo()
            ))
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    std::fs::create_dir(f.0.join("workspace")).unwrap();
    std::os::unix::fs::symlink(&outside.0, f.0.join("workspace/escape")).unwrap();
    let workspace = Workspace::open(&f.0.join("workspace")).unwrap();
    assert_eq!(
        workspace.repo("escape").unwrap_err().kind,
        reviewizard::utils::errors::ErrorKind::BadRequest
    );
    assert_eq!(
        workspace.repo("..").unwrap_err().kind,
        reviewizard::utils::errors::ErrorKind::BadRequest
    );
    assert!(workspace.list().unwrap().is_empty());
}

#[tokio::test]
async fn services_can_run_directly_without_http_and_preserve_validation() {
    use reviewizard::{
        services::{branches, commits, files, repos},
        utils::errors::ErrorKind,
    };
    let f = Fixture::new();
    let workspace = Workspace::open(&f.0).unwrap();
    let repo = f.repo();
    let query = serde_json::from_value(
        serde_json::json!({"repo": repo, "ref": "HEAD", "path": "a.txt", "side": "index"}),
    )
    .unwrap();
    let file = files::file(&workspace, query).await.unwrap();
    assert_eq!(serde_json::to_value(file).unwrap()["content"], "staged\n");
    let query = serde_json::from_value(
        serde_json::json!({"repo": repo, "ref": "HEAD", "path": "image.png"}),
    )
    .unwrap();
    let blob = files::blob(&workspace, query).await.unwrap();
    assert_eq!(blob.content, [0x89, b'P', b'N', b'G', 0, 0xff]);
    let query = serde_json::from_value(serde_json::json!({"repo": repo})).unwrap();
    let branches = branches::list(&workspace, query).await.unwrap();
    assert_eq!(
        serde_json::to_value(branches).unwrap()["branches"][0]["name"],
        "main"
    );
    let query = serde_json::from_value(
        serde_json::json!({"repo": repo, "ref": "HEAD", "base": "missing-base", "limit": "1"}),
    )
    .unwrap();
    let commits = commits::list(&workspace, query).await.unwrap();
    assert_eq!(
        serde_json::to_value(commits).unwrap()["commits"][0]["subject"],
        "feature"
    );
    let query = serde_json::from_value(serde_json::json!({"repo": repo, "ref": "-bad"})).unwrap();
    match files::list(&workspace, query).await {
        Err(error) => assert_eq!(error.kind, ErrorKind::BadRequest),
        Ok(_) => panic!("flag-like ref must be rejected before running git"),
    }
    assert_eq!(
        serde_json::to_value(repos::list(workspace).await.unwrap()).unwrap()["repos"][0]["name"],
        repo
    );
}

#[test]
fn shared_path_validation_handles_normalization_and_git_metadata() {
    use reviewizard::utils::validation::{valid_ref, validate_path};
    for path in [
        "src/file.rs",
        "src/../a.txt",
        "dir with spaces/file",
        "./a.txt",
    ] {
        assert!(validate_path(path).is_ok(), "{path}");
    }
    for path in [
        "",
        "../secret",
        "a/../../secret",
        "/etc/passwd",
        ".git/config",
        "src/../.GIT/config",
        "nul\0file",
    ] {
        assert!(validate_path(path).is_err(), "{path}");
    }
    assert!(valid_ref("feature/review").is_ok());
    assert!(valid_ref("-bad").is_err());
    assert!(valid_ref("").is_err());
}

fn diff_file<'a>(value: &'a serde_json::Value, path: &str) -> &'a serde_json::Value {
    value["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["newPath"] == path || f["oldPath"] == path)
        .unwrap()
}
fn added_content(file: &serde_json::Value) -> Vec<String> {
    file["hunks"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|h| h["lines"].as_array().unwrap())
        .filter(|l| l["kind"] == "add")
        .map(|l| l["content"].as_str().unwrap().to_owned())
        .collect()
}
#[tokio::test]
async fn diffs_distinguish_committed_staged_and_combined_worktree() {
    let f = Fixture::new();
    let url = format!("/api/diff?repo={}&head=HEAD&base=main", f.repo());
    let committed = f.json(&url).await;
    assert_eq!(committed["head"], "HEAD");
    assert_eq!(committed["base"].as_str().unwrap().len(), 40);
    assert_eq!(committed["files"].as_array().unwrap().len(), 1);
    assert_eq!(diff_file(&committed, "feature.txt")["status"], "added");
    let staged = f
        .json(&format!("{url}&uncommitted=true&unstaged=false"))
        .await;
    assert_eq!(added_content(diff_file(&staged, "a.txt")), ["staged"]);
    assert!(
        !staged["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["newPath"] == "untracked.txt")
    );
    let working = f.json(&format!("{url}&uncommitted=true")).await;
    assert_eq!(added_content(diff_file(&working, "a.txt")), ["working"]);
    assert_eq!(added_content(diff_file(&working, "untracked.txt")), ["new"]);
    let line = &diff_file(&working, "untracked.txt")["hunks"][0]["lines"][0];
    assert_eq!(line["newLine"], 1);
    assert!(line.get("oldLine").is_none());
    let empty = f
        .json(&format!("/api/diff?repo={}&head=main&base=main", f.repo()))
        .await;
    assert!(empty["files"].is_null());
    let fallback = f
        .json(&format!(
            "/api/diff?repo={}&head=HEAD&base=deleted-branch",
            f.repo()
        ))
        .await;
    assert_eq!(fallback, committed);
}
#[tokio::test]
async fn selected_from_commit_includes_its_changes_and_root_uses_empty_tree() {
    let f = Fixture::new();
    let selected = f
        .json(&format!("/api/diff?repo={}&head=HEAD&from=HEAD", f.repo()))
        .await;
    assert_eq!(selected["files"].as_array().unwrap().len(), 1);
    assert_eq!(diff_file(&selected, "feature.txt")["status"], "added");
    let root = f
        .json(&format!("/api/diff?repo={}&head=main&from=main", f.repo()))
        .await;
    assert_eq!(root["base"], reviewizard::services::git::refs::EMPTY_TREE);
    assert_eq!(root["files"].as_array().unwrap().len(), 2);
    for suffix in [
        "head=-bad",
        "head=HEAD&base=-bad",
        "head=HEAD&from=-bad",
        "head=HEAD&from=missing",
    ] {
        assert_eq!(
            f.get(&format!("/api/diff?repo={}&{suffix}", f.repo()))
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(&f.0)
        .args(["rev-parse", "main^{tree}"])
        .output()
        .unwrap();
    let tree = String::from_utf8(output.stdout).unwrap();
    let orphan = std::process::Command::new("git")
        .arg("-C")
        .arg(&f.0)
        .args(["commit-tree", tree.trim(), "-m", "unrelated"])
        .output()
        .unwrap();
    assert!(orphan.status.success());
    let sha = String::from_utf8(orphan.stdout).unwrap();
    f.git(&["branch", "unrelated", sha.trim()]);
    let response = f
        .get(&format!(
            "/api/diff?repo={}&head=HEAD&base=unrelated",
            f.repo()
        ))
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = axum::body::to_bytes(response.into_body(), 10000)
        .await
        .unwrap();
    assert!(String::from_utf8_lossy(&body).contains("share no common history"));
}
#[tokio::test]
async fn diff_attributes_untracked_limits_binary_and_safe_symlinks() {
    let f = Fixture::new();
    std::fs::write(f.0.join(".gitattributes"), "*.txt linguist-language=Rust\nmanual.txt linguist-generated=true\nCargo.lock linguist-generated=false\n").unwrap();
    std::fs::write(f.0.join("manual.txt"), "manual\n").unwrap();
    std::fs::write(f.0.join("Cargo.lock"), "lock\n").unwrap();
    std::fs::write(f.0.join("yarn.lock"), "lock\n").unwrap();
    std::fs::write(f.0.join("empty.txt"), "").unwrap();
    std::fs::write(f.0.join("binary.dat"), [0, 1, 2]).unwrap();
    std::fs::write(f.0.join("big.txt"), vec![b'x'; (1 << 20) + 1]).unwrap();
    std::fs::write(f.0.join("cap.txt"), vec![b'x'; 1 << 20]).unwrap();
    let mut bytes = vec![b'x'; (1 << 20) + 1];
    bytes[0] = 0;
    std::fs::write(f.0.join("big.bin"), bytes).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(f.0.join(".git/config"), f.0.join("secret-link")).unwrap();
    let result = f
        .json(&format!(
            "/api/diff?repo={}&head=HEAD&base=main&uncommitted=true",
            f.repo()
        ))
        .await;
    assert_eq!(diff_file(&result, "manual.txt")["language"], "Rust");
    assert_eq!(diff_file(&result, "manual.txt")["generated"], true);
    assert!(diff_file(&result, "Cargo.lock").get("generated").is_none());
    assert_eq!(diff_file(&result, "yarn.lock")["generated"], true);
    assert_eq!(
        diff_file(&result, "empty.txt")["hunks"],
        serde_json::json!([])
    );
    assert_eq!(diff_file(&result, "binary.dat")["binary"], true);
    assert_eq!(diff_file(&result, "big.txt")["tooLarge"], true);
    assert_eq!(
        diff_file(&result, "big.txt")["hunks"],
        serde_json::json!([])
    );
    assert_eq!(diff_file(&result, "big.bin")["binary"], true);
    assert!(diff_file(&result, "big.bin").get("tooLarge").is_none());
    assert_eq!(
        diff_file(&result, "cap.txt")["hunks"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(
        !result["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["newPath"] == "secret-link")
    );
}
#[tokio::test]
async fn tracked_renames_deletions_binary_and_unicode_paths() {
    let f = Fixture::new();
    f.git(&["mv", "feature.txt", "renamed file.txt"]);
    std::fs::remove_file(f.0.join("a.txt")).unwrap();
    std::fs::write(f.0.join("image.png"), [0, 4, 5]).unwrap();
    std::fs::write(f.0.join("árvíz.txt"), "Unicode\n").unwrap();
    f.git(&["add", "-A"]);
    let result = f
        .json(&format!(
            "/api/diff?repo={}&head=HEAD&base=HEAD&uncommitted=true&unstaged=false",
            f.repo()
        ))
        .await;
    let renamed = diff_file(&result, "renamed file.txt");
    assert_eq!(renamed["status"], "renamed");
    assert_eq!(renamed["oldPath"], "feature.txt");
    assert_eq!(diff_file(&result, "a.txt")["status"], "deleted");
    assert_eq!(diff_file(&result, "image.png")["binary"], true);
    assert_eq!(added_content(diff_file(&result, "árvíz.txt")), ["Unicode"]);
}
#[test]
fn diff_parser_handles_header_like_content_multiple_hunks_and_limits() {
    use reviewizard::services::git::diff::parse;
    let patch = "diff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -1,2 +1,2 @@\n--- old\n+++ new\n same\n@@ -10 +10 @@ heading -3 +4\n-old\n+new\n\\ No newline at end of file\n";
    let value = serde_json::to_value(parse(patch).unwrap()).unwrap();
    assert_eq!(value[0]["oldPath"], "f");
    assert_eq!(value[0]["hunks"][0]["lines"][0]["content"], "-- old");
    assert_eq!(value[0]["hunks"][1]["lines"][0]["oldLine"], 10);
    assert_eq!(value[0]["hunks"][1]["lines"].as_array().unwrap().len(), 2);
    let modes = parse("diff --git a/bin b/bin\nold mode 100644\nnew mode 100755\n").unwrap();
    assert_eq!(modes[0].old_path, "bin");
    assert!(modes[0].hunks.is_empty());
    let binary = parse("diff --git a/new.bin b/new.bin\nnew file mode 100644\nBinary files /dev/null and b/new.bin differ\n").unwrap();
    assert!(binary[0].old_path.is_empty());
    assert!(binary[0].binary);
    assert!(parse(&"x".repeat(16 * 1024 * 1024)).is_err());
}

#[tokio::test]
async fn diff_without_a_trunk_requires_explicit_base() {
    let f = Fixture::new();
    f.git(&["branch", "-m", "main", "trunk"]);
    let result = f
        .get(&format!("/api/diff?repo={}&head=HEAD", f.repo()))
        .await;
    assert_eq!(result.status(), StatusCode::BAD_REQUEST);
    let body = axum::body::to_bytes(result.into_body(), 10000)
        .await
        .unwrap();
    assert!(String::from_utf8_lossy(&body).contains("select a base branch"));
    let result = f
        .json(&format!("/api/diff?repo={}&head=HEAD&base=trunk", f.repo()))
        .await;
    assert_eq!(result["files"].as_array().unwrap().len(), 1);
}

#[cfg(unix)]
#[tokio::test]
async fn diff_disables_configured_external_diff_and_textconv() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    let helper = f.0.join("diff-helper");
    std::fs::write(&helper, "#!/bin/sh\ntouch helper-was-run\nexit 1\n").unwrap();
    std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o755)).unwrap();
    f.git(&["config", "diff.external", helper.to_str().unwrap()]);
    f.git(&["config", "diff.custom.textconv", helper.to_str().unwrap()]);
    std::fs::write(f.0.join(".gitattributes"), "*.txt diff=custom\n").unwrap();
    let result = f
        .json(&format!("/api/diff?repo={}&head=HEAD&base=main", f.repo()))
        .await;
    assert_eq!(
        added_content(diff_file(&result, "feature.txt")),
        ["feature"]
    );
    assert!(!f.0.join("helper-was-run").exists());
}

impl Fixture {
    fn sha(&self) -> String {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&self.0)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        assert!(out.status.success());
        String::from_utf8(out.stdout).unwrap().trim().into()
    }
    fn commit(&self) -> String {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-m", "review fixture"]);
        self.sha()
    }
}
fn anchor(
    path: &str,
    line: i64,
    end: i64,
    snippet: &str,
    sha: &str,
    side: reviewizard::services::store::Side,
) -> reviewizard::services::store::Comment {
    reviewizard::services::store::Comment {
        file_path: path.into(),
        start_line: line,
        end_line: end,
        snippet: snippet.into(),
        commit_sha: sha.into(),
        side,
        ..Default::default()
    }
}
#[tokio::test]
async fn review_snippets_respect_sides_and_capture_bounds() {
    use reviewizard::services::{git::Repo, review, store::Side};
    let f = Fixture::new();
    let repo = Repo { path: f.0.clone() };
    for (side, expected) in [
        (Side::Head, "committed"),
        (Side::Index, "staged"),
        (Side::Worktree, "working"),
    ] {
        assert_eq!(
            review::capture_snippet(Some(&repo), "HEAD", "a.txt", 1, 99, side).await,
            expected
        );
    }
    assert_eq!(
        review::capture_snippet(Some(&repo), "HEAD", "a.txt", 0, 0, Side::Head).await,
        ""
    );
    assert_eq!(
        review::capture_snippet(Some(&repo), "HEAD", "a.txt", 99, 99, Side::Head).await,
        ""
    );
    assert_eq!(
        review::capture_snippet(None, "HEAD", "a.txt", 1, 1, Side::Head).await,
        ""
    );
}
#[tokio::test]
async fn diff_anchors_follow_moves_renames_and_invalidate_edited_blocks() {
    use reviewizard::services::{
        review,
        store::{Review, Side},
    };
    let f = Fixture::new();
    for (path, text) in [
        ("shift.txt", "shift one\ntwo\nthree\n"),
        ("rename.txt", "rename one\ntwo\nthree\n"),
        ("edit.txt", "edit one\ntwo\nthree\n"),
        ("deleted.txt", "delete one\ntwo\nthree\n"),
        ("stable.txt", "stable one\ntwo\nthree\n"),
    ] {
        std::fs::write(f.0.join(path), text).unwrap();
    }
    let old = f.commit();
    std::fs::write(f.0.join("shift.txt"), "prelude\nshift one\ntwo\nthree\n").unwrap();
    f.git(&["mv", "rename.txt", "new-name.txt"]);
    std::fs::write(f.0.join("edit.txt"), "edit one\nEDITED\nthree\n").unwrap();
    std::fs::remove_file(f.0.join("deleted.txt")).unwrap();
    f.commit();
    let original = vec![
        anchor("shift.txt", 1, 2, "shift one\ntwo", &old, Side::Head),
        anchor("rename.txt", 2, 2, "two", &old, Side::Head),
        anchor("edit.txt", 1, 3, "edit one\ntwo\nthree", &old, Side::Head),
        anchor("deleted.txt", 1, 1, "delete one", &old, Side::Head),
        anchor("stable.txt", 2, 2, "two", &old, Side::Head),
    ];
    let cache = review::DiffCache::new();
    let mut r = Review {
        repo_path: f.0.to_string_lossy().into(),
        head_ref: "HEAD".into(),
        comments: original.clone(),
        ..Default::default()
    };
    review::annotate(&mut r, None, Some(&cache)).await;
    assert_eq!(
        r.comments
            .iter()
            .map(|c| c.anchor_status.as_str())
            .collect::<Vec<_>>(),
        ["moved", "moved", "outdated", "outdated", "current"]
    );
    assert_eq!(r.comments[0].current_start_line, 2);
    assert_eq!(r.comments[0].current_end_line, 3);
    assert_eq!(r.comments[1].current_file_path, "new-name.txt");
    assert_eq!(r.comments[1].current_start_line, 2);
    // Single-comment scoped diffs must agree with the whole-tree optimization.
    for (index, c) in original.into_iter().enumerate() {
        let mut one = Review {
            repo_path: r.repo_path.clone(),
            head_ref: "HEAD".into(),
            comments: vec![c],
            ..Default::default()
        };
        review::annotate(&mut one, None, Some(&cache)).await;
        assert_eq!(
            one.comments[0].anchor_status,
            r.comments[index].anchor_status
        );
        assert_eq!(
            one.comments[0].current_file_path,
            r.comments[index].current_file_path
        );
    }
    let mut huge = anchor("shift.txt", 1, 2_000_000_000, "shift one", &old, Side::Head);
    let repo = reviewizard::services::git::Repo { path: f.0.clone() };
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        review::annotate_comment(Some(&repo), "HEAD", "", &mut huge),
    )
    .await
    .unwrap();
    assert_eq!(huge.anchor_status, "moved");
}
#[tokio::test]
async fn snippet_fallback_is_side_specific_and_ambiguous_matches_are_outdated() {
    use reviewizard::services::{
        review,
        store::{Review, Side},
    };
    let f = Fixture::new();
    std::fs::write(f.0.join("snip.txt"), "a\nneedle\nb\n").unwrap();
    let old = f.commit();
    std::fs::write(f.0.join("snip.txt"), "a\nneedle\nb\nSTAGED\n").unwrap();
    f.git(&["add", "snip.txt"]);
    std::fs::write(f.0.join("snip.txt"), "new\na\nneedle\nb\n").unwrap();
    let mut r = Review {
        repo_path: f.0.to_string_lossy().into(),
        head_ref: "HEAD".into(),
        comments: vec![
            anchor("snip.txt", 2, 2, "needle", &old, Side::Head),
            anchor("snip.txt", 2, 2, "needle", &old, Side::Index),
            anchor("snip.txt", 2, 2, "needle", &old, Side::Worktree),
        ],
        ..Default::default()
    };
    let cache = review::DiffCache::new();
    review::annotate(&mut r, None, Some(&cache)).await;
    assert_eq!(
        r.comments
            .iter()
            .map(|c| c.anchor_status.as_str())
            .collect::<Vec<_>>(),
        ["current", "current", "moved"]
    );
    assert_eq!(r.comments[2].current_start_line, 3);
    std::fs::write(f.0.join("snip.txt"), "new\na\nneedle\nneedle\n").unwrap();
    review::annotate(&mut r, None, Some(&cache)).await;
    assert_eq!(r.comments[2].anchor_status, "outdated");
    assert_eq!(r.comments[2].current_start_line, 0);
    r.comments.push(anchor("missing", 0, 0, "", "", Side::Head));
    review::annotate(&mut r, None, None).await;
    assert_eq!(r.comments.last().unwrap().anchor_status, "current");
}
#[tokio::test]
async fn reviewed_hashes_recheck_bytes_and_blocked_repositories_preserve_state() {
    use reviewizard::services::{
        git::Repo,
        review,
        store::{Review, ReviewedFile, Side},
    };
    let f = Fixture::new();
    let repo = Repo { path: f.0.clone() };
    let paths = vec!["a.txt".into(), "gone".into(), "".into()];
    let hashes = review::fingerprint_files(&repo, "HEAD", &paths, Side::Worktree).await;
    assert_eq!(hashes["gone"], "absent");
    assert!(!hashes.contains_key(""));
    let staged = review::fingerprint_files(&repo, "HEAD", &paths, Side::Index).await;
    assert_ne!(hashes["a.txt"], staged["a.txt"]);
    let marks = vec![
        ReviewedFile {
            path: "a.txt".into(),
            content_hash: hashes["a.txt"].clone(),
            side: Side::Worktree,
        },
        ReviewedFile {
            path: "gone".into(),
            content_hash: "absent".into(),
            side: Side::Worktree,
        },
        ReviewedFile {
            path: "legacy".into(),
            content_hash: "".into(),
            side: Side::Head,
        },
    ];
    let mut r = Review {
        repo_path: f.0.to_string_lossy().into(),
        head_ref: "HEAD".into(),
        reviewed_files: vec!["a.txt".into(), "gone".into(), "legacy".into()],
        ..Default::default()
    };
    review::annotate(&mut r, Some(&marks), None).await;
    assert_eq!(r.reviewed_files, ["a.txt", "gone", "legacy"]);
    std::fs::write(f.0.join("a.txt"), "changed").unwrap();
    std::fs::write(f.0.join("gone"), "returned").unwrap();
    review::annotate(&mut r, Some(&marks), None).await;
    assert_eq!(r.reviewed_files, ["legacy"]);
    r.reviewed_files = vec!["keep on store failure".into()];
    review::annotate(&mut r, None, None).await;
    assert_eq!(r.reviewed_files, ["keep on store failure"]);
    r.head_ref = "deleted-branch".into();
    review::annotate(&mut r, Some(&marks), None).await;
    assert!(r.annotation_error.contains("no longer resolves"));
    assert_eq!(r.reviewed_files, ["keep on store failure"]);
}

#[tokio::test]
async fn review_read_and_exports_annotate_and_preserve_http_contracts() {
    use reviewizard::services::store::{Comment, Side};
    let f = Fixture::new();
    let store = Store::in_memory().unwrap();
    let r = store
        .create_or_get_review(f.0.to_str().unwrap(), "main", "HEAD", &f.sha())
        .unwrap();
    store.set_review_summary(r.id, "Ready for review").unwrap();
    let c = store
        .add_comment(Comment {
            review_id: r.id,
            file_path: "a.txt".into(),
            start_line: 1,
            end_line: 1,
            snippet: "committed".into(),
            side: Side::Head,
            comment_type: "bug".into(),
            body: "Fix this".into(),
            author: "human".into(),
            ..Default::default()
        })
        .unwrap();
    store.add_reply(c.id, "I will check", "agent").unwrap();
    let state = Arc::new(AppState {
        workspace: Workspace::open(&f.0).unwrap(),
        store: store.clone(),
        diffs: Default::default(),
        live: Default::default(),
    });
    let app = http::router(state);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/reviews/{}", r.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 100000)
        .await
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["comments"][0]["anchorStatus"], "current");
    assert_eq!(value["comments"][0]["replies"][0]["body"], "I will check");
    for (suffix, is_json) in [("export", true), ("export.md", false)] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/reviews/{}/{suffix}?instructions=true", r.id))
                    .header("host", "127.0.0.1:7778")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        if !is_json {
            assert_eq!(
                response.headers()["content-type"],
                "text/markdown; charset=utf-8"
            );
            assert!(
                response.headers()["content-disposition"]
                    .to_str()
                    .unwrap()
                    .contains("code-review-HEAD-")
            );
        }
        let bytes = axum::body::to_bytes(response.into_body(), 100000)
            .await
            .unwrap();
        let markdown = if is_json {
            let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert!(value["filename"].as_str().unwrap().ends_with(".md"));
            value["markdown"].as_str().unwrap().to_owned()
        } else {
            String::from_utf8(bytes.to_vec()).unwrap()
        };
        assert!(markdown.contains("Ready for review"));
        assert!(markdown.contains(&format!("/api/comments/{}/replies", c.id)));
        assert!(markdown.contains("127.0.0.1:7778"));
    }
    assert_eq!(store.get_review(r.id).unwrap().status, "exported");
    // Derived status is never persisted as an anchor field.
    assert!(store.get_comment(c.id).unwrap().anchor_status.is_empty());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/reviews/999/export")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn batch_parser_keeps_binary_payloads_and_rejects_partial_records() {
    use reviewizard::services::git::batch;
    let specs = vec![
        "HEAD:a space".into(),
        "HEAD:empty".into(),
        "HEAD:missing".into(),
    ];
    let bytes = b"abc blob 3\nx\0y\ndef blob 0\n\nHEAD:missing missing\n";
    let result = batch::parse(bytes, &specs).unwrap();
    assert_eq!(result["HEAD:a space"], b"x\0y");
    assert!(result["HEAD:empty"].is_empty());
    assert!(!result.contains_key("HEAD:missing"));
    for bytes in [&b"abc blob 99\nx\n"[..], &b"abc blob 1\nx"[..], &b""[..]] {
        assert!(batch::parse(bytes, &specs).is_err());
    }
}

async fn send_json(
    app: &axum::Router,
    method: &str,
    path: &str,
    value: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(value.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let code = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 8 << 20)
        .await
        .unwrap();
    (
        code,
        if bytes.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        },
    )
}
#[tokio::test]
async fn mutation_endpoints_publish_after_success_and_preserve_anchors() {
    use reviewizard::services::live::LiveUpdates;
    let f = Fixture::new();
    let store = Store::in_memory().unwrap();
    let live = LiveUpdates::default();
    let app = http::router(Arc::new(AppState {
        workspace: Workspace::open(&f.0).unwrap(),
        store: store.clone(),
        diffs: Default::default(),
        live: live.clone(),
    }));
    let (code, r) = send_json(
        &app,
        "POST",
        "/api/reviews",
        serde_json::json!({"repo":f.repo(),"head":"HEAD","base":"main"}),
    )
    .await;
    assert_eq!(code, StatusCode::OK);
    let rid = r["id"].as_i64().unwrap();
    let mut sub = live.subscribe(rid, None);
    let (code,c)=send_json(&app,"POST",&format!("/api/reviews/{rid}/comments"),serde_json::json!({"filePath":"a.txt","startLine":1,"endLine":1,"body":"fix","side":"index"})).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(c["snippet"], "staged");
    assert_eq!(c["author"], "agent");
    assert_eq!(c["type"], "general");
    assert_eq!(sub.next().await, Some("meta"));
    let cid = c["id"].as_i64().unwrap();
    let old_sha = c["commitSha"].clone();
    std::fs::write(f.0.join("a.txt"), "changed\n").unwrap();
    f.git(&["add", "a.txt"]);
    let (code, edited) = send_json(
        &app,
        "PATCH",
        &format!("/api/comments/{cid}"),
        serde_json::json!({"body":"wording only"}),
    )
    .await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(edited["snippet"], "staged");
    assert_eq!(edited["commitSha"], old_sha);
    assert_eq!(edited["anchorStatus"], "outdated");
    assert_eq!(sub.next().await, Some("meta"));
    let (code, edited) = send_json(
        &app,
        "PATCH",
        &format!("/api/comments/{cid}"),
        serde_json::json!({"startLine":1,"endLine":1}),
    )
    .await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(edited["snippet"], "changed");
    assert_eq!(edited["anchorStatus"], "current");
    assert_eq!(sub.next().await, Some("meta"));
    let (code, reply) = send_json(
        &app,
        "POST",
        &format!("/api/comments/{cid}/replies"),
        serde_json::json!({"body":"checking"}),
    )
    .await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(sub.next().await, Some("meta"));
    let reply_id = reply["id"].as_i64().unwrap();
    for (method, path, body) in [
        (
            "PATCH",
            format!("/api/replies/{reply_id}"),
            serde_json::json!({"body":"done"}),
        ),
        (
            "POST",
            format!("/api/comments/{cid}/resolved"),
            serde_json::json!({"resolved":true}),
        ),
        (
            "POST",
            format!("/api/reviews/{rid}/summary"),
            serde_json::json!({"summary":" summary "}),
        ),
        (
            "POST",
            format!("/api/reviews/{rid}/reviewed"),
            serde_json::json!({"filePaths":["a.txt"],"reviewed":true,"side":"index"}),
        ),
        (
            "DELETE",
            format!("/api/replies/{reply_id}"),
            serde_json::Value::Null,
        ),
        (
            "DELETE",
            format!("/api/comments/{cid}"),
            serde_json::Value::Null,
        ),
        (
            "POST",
            format!("/api/reviews/{rid}/reset"),
            serde_json::Value::Null,
        ),
    ] {
        let (code, _) = send_json(&app, method, &path, body).await;
        assert!(code.is_success(), "{path}");
        assert_eq!(sub.next().await, Some("meta"));
    }
    let review = store.get_review(rid).unwrap();
    assert!(review.comments.is_empty());
    assert!(review.reviewed_files.is_empty());
    assert!(review.summary.is_empty());
    let (code, _) = send_json(
        &app,
        "POST",
        &format!("/api/reviews/{rid}/comments"),
        serde_json::json!({"filePath":"../escape","body":"bad"}),
    )
    .await;
    assert_eq!(code, StatusCode::BAD_REQUEST);
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(20), sub.next())
            .await
            .is_err()
    );
    let (code, _) = send_json(
        &app,
        "POST",
        "/api/comments/999/replies",
        serde_json::json!({"body":"missing"}),
    )
    .await;
    assert_eq!(code, StatusCode::NOT_FOUND);
}
#[tokio::test(start_paused = true)]
async fn sse_connected_keepalive_meta_and_shutdown_frames() {
    use http_body_util::BodyExt;
    let store = Store::in_memory().unwrap();
    let state = Arc::new(AppState {
        workspace: Workspace::open(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap(),
        store,
        diffs: Default::default(),
        live: Default::default(),
    });
    let response = http::router(state.clone())
        .oneshot(
            Request::builder()
                .uri("/api/reviews/123/events")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "text/event-stream");
    assert_eq!(response.headers()["cache-control"], "no-cache");
    let mut body = response.into_body();
    let first = body.frame().await.unwrap().unwrap().into_data().unwrap();
    assert!(String::from_utf8_lossy(&first).contains(": connected"));
    let heartbeat = body.frame().await.unwrap().unwrap().into_data().unwrap();
    assert!(String::from_utf8_lossy(&heartbeat).contains(": keepalive"));
    state.live.publish(123, false, false);
    let meta = body.frame().await.unwrap().unwrap().into_data().unwrap();
    assert!(String::from_utf8_lossy(&meta).contains("data: meta"));
    state.live.shutdown();
    assert!(body.frame().await.is_none());
}
#[tokio::test]
async fn fingerprints_detect_changes_without_optional_index_locks() {
    use reviewizard::services::git::{Repo, fingerprint};
    let f = Fixture::new();
    let repo = Repo { path: f.0.clone() };
    let refs = fingerprint::refs(&repo).await.unwrap();
    let work = fingerprint::worktree(&repo).await.unwrap();
    std::fs::write(f.0.join("new file.txt"), "new\n").unwrap();
    assert_ne!(work, fingerprint::worktree(&repo).await.unwrap());
    assert_eq!(refs, fingerprint::refs(&repo).await.unwrap());
    f.git(&["commit", "--allow-empty", "-m", "empty"]);
    assert_ne!(refs, fingerprint::refs(&repo).await.unwrap());
    let refs = fingerprint::refs(&repo).await.unwrap();
    f.git(&["checkout", "--detach"]);
    assert_ne!(refs, fingerprint::refs(&repo).await.unwrap());
    assert!(!f.0.join(".git/index.lock").exists());
}
#[tokio::test]
async fn polling_emits_diff_then_refs_for_real_edits_and_commits() {
    use reviewizard::services::live::LiveUpdates;
    let f = Fixture::new();
    let live = LiveUpdates::default();
    let mut sub = live.subscribe(1, Some(f.0.clone()));
    // Keep changing the worktree until a post-baseline tick observes an edit.
    // This avoids assuming a Git subprocess completes within a fixed sleep on CI.
    let edits = async {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(250));
        let mut iteration = 0;
        loop {
            interval.tick().await;
            iteration += 1;
            tokio::fs::write(f.0.join("a.txt"), format!("poll update {iteration}\n"))
                .await
                .unwrap();
        }
    };
    tokio::select! {
        kind = tokio::time::timeout(std::time::Duration::from_secs(12), sub.next()) => assert_eq!(kind.unwrap(), Some("diff")),
        _ = edits => unreachable!(),
    }
    f.commit();
    let kind = tokio::time::timeout(std::time::Duration::from_secs(8), sub.next())
        .await
        .unwrap();
    assert_eq!(kind, Some("refs"));
    live.shutdown();
    assert_eq!(sub.next().await, None);
}

#[tokio::test]
async fn mutation_validation_filtering_and_oversized_requests_do_not_notify() {
    use reviewizard::services::live::LiveUpdates;
    let f = Fixture::new();
    let store = Store::in_memory().unwrap();
    let live = LiveUpdates::default();
    let app = http::router(Arc::new(AppState {
        workspace: Workspace::open(&f.0).unwrap(),
        store: store.clone(),
        diffs: Default::default(),
        live: live.clone(),
    }));
    let (code, r) = send_json(
        &app,
        "POST",
        "/api/reviews",
        serde_json::json!({"repo":f.repo(),"head":"HEAD","base":"stale-base"}),
    )
    .await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(r["baseRef"], "main");
    let id = r["id"].as_i64().unwrap();
    let mut sub = live.subscribe(id, None);
    for value in [
        serde_json::json!({"filePath":"a.txt","body":" "}),
        serde_json::json!({"filePath":"a.txt","body":"x","startLine":-1}),
        serde_json::json!({"filePath":"a.txt","body":"x","type":"wrong"}),
        serde_json::json!({"filePath":"a.txt","body":"x","side":"staged"}),
    ] {
        assert_eq!(
            send_json(&app, "POST", &format!("/api/reviews/{id}/comments"), value)
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
    let huge = serde_json::json!({"summary":"x".repeat((8<<20)+1)});
    assert_eq!(
        send_json(&app, "POST", &format!("/api/reviews/{id}/summary"), huge)
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(20), sub.next())
            .await
            .is_err()
    );
    for author in ["agent", "reviewer"] {
        assert_eq!(
            send_json(
                &app,
                "POST",
                &format!("/api/reviews/{id}/comments"),
                serde_json::json!({"filePath":"a.txt","body":"note","author":author})
            )
            .await
            .0,
            StatusCode::OK
        );
    }
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/reviews/{id}/comments?author=reviewer"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 100000)
        .await
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["comments"].as_array().unwrap().len(), 1);
    assert_eq!(value["comments"][0]["author"], "reviewer");
}

#[tokio::test]
async fn backend_only_serves_api_and_health() {
    for path in ["/", "/review/123", "/assets/index.js", "/api/not-a-route"] {
        let response = app()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        assert_eq!(response.headers()["content-type"], "application/json");
    }
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

mod git_cases;

mod persistence;

#[tokio::test]
async fn saved_review_scopes_preserve_source_sides_and_do_not_reuse_legacy_reviews() {
    use serde_json::json;
    let f = Fixture::new();
    let app = f.app();
    let (code, project) = send_json(&app, "POST", "/api/projects", json!({"path": f.0})).await;
    assert_eq!(code, StatusCode::OK, "{project}");
    assert_eq!(project["path"], f.0.to_string_lossy().as_ref());
    let pid = project["id"].as_i64().unwrap();
    let endpoint = format!("/api/projects/{pid}/reviews");
    let mut ids = Vec::new();
    for (side, expected, file) in [
        ("head", "feature", "feature.txt"),
        ("index", "staged", "a.txt"),
        ("worktree", "working", "a.txt"),
    ] {
        let (code, r) = send_json(
            &app,
            "POST",
            &endpoint,
            json!({"head":"feature/test", "base":"main", "from":"all", "side":side}),
        )
        .await;
        assert_eq!(code, StatusCode::OK, "{r}");
        assert_eq!(r["scopeSide"], side);
        assert_eq!(r["scopeFrom"], "all");
        assert_eq!(r["repoPath"], project["path"]);
        let id = r["id"].as_i64().unwrap();
        assert!(!ids.contains(&id));
        ids.push(id);
        let (code, diff) =
            send_json(&app, "GET", &format!("/api/reviews/{id}/diff"), json!(null)).await;
        assert_eq!(code, StatusCode::OK, "{diff}");
        assert!(diff.to_string().contains(expected));
        if side == "head" {
            assert!(!diff.to_string().contains("working"));
        }
        if side == "index" {
            assert!(!diff.to_string().contains("working"));
        }
        let (code, comment) = send_json(
            &app,
            "POST",
            &format!("/api/reviews/{id}/comments"),
            json!({"filePath":file,"startLine":1,"endLine":1,"side":side,"body":"saved scope"}),
        )
        .await;
        assert_eq!(code, StatusCode::OK, "{comment}");
        assert!(comment["snippet"].as_str().unwrap().contains(expected));
    }
    let (code, _) = send_json(
        &app,
        "POST",
        &format!("/api/reviews/{}/comments", ids[1]),
        json!({"filePath":"a.txt","startLine":1,"endLine":1,"side":"head","body":"wrong side"}),
    )
    .await;
    assert_eq!(code, StatusCode::BAD_REQUEST);
    for body in [
        json!({"head":"missing","base":"main","side":"head"}),
        json!({"head":"feature/test","base":"missing","side":"head"}),
        json!({"head":"main","base":"main","from":"feature/test","side":"head"}),
        json!({"head":"main","base":"main","side":"worktree"}),
    ] {
        let (code, error) = send_json(&app, "POST", &endpoint, body).await;
        assert_eq!(code, StatusCode::BAD_REQUEST, "{error}");
    }
    let (_, list) = send_json(&app, "GET", &endpoint, json!(null)).await;
    assert_eq!(list.as_array().unwrap().len(), 3);
    let (code, legacy) = send_json(
        &app,
        "POST",
        "/api/reviews",
        json!({"repo":format!("@project/{pid}"),"head":"feature/test","base":"main"}),
    )
    .await;
    assert_eq!(code, StatusCode::OK);
    assert!(!ids.contains(&legacy["id"].as_i64().unwrap()));
    let id = ids[0];
    let (code, _) = send_json(
        &app,
        "POST",
        &format!("/api/reviews/{id}/status"),
        json!({"status":"completed"}),
    )
    .await;
    assert_eq!(code, StatusCode::NO_CONTENT);
    let (code, _) = send_json(
        &app,
        "POST",
        &format!("/api/reviews/{id}/export"),
        json!(null),
    )
    .await;
    assert_eq!(code, StatusCode::OK);
    let (_, completed) = send_json(&app, "GET", &format!("/api/reviews/{id}"), json!(null)).await;
    assert_eq!(completed["status"], "completed");
    f.git(&["switch", "main"]);
    let (code, _) = send_json(
        &app,
        "GET",
        &format!("/api/reviews/{}/diff", ids[1]),
        json!(null),
    )
    .await;
    assert_eq!(code, StatusCode::BAD_REQUEST);
    let (code, _) = send_json(
        &app,
        "POST",
        &format!("/api/reviews/{}/comments", ids[1]),
        json!({"filePath":"a.txt","startLine":1,"endLine":1,"side":"index","body":"wrong branch"}),
    )
    .await;
    assert_eq!(code, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn codebase_reviews_cover_unchanged_files_and_preserve_source_sides() {
    use serde_json::json;
    let f = Fixture::new();
    let app = f.app();
    let (code, project) = send_json(&app, "POST", "/api/projects", json!({"path": f.0})).await;
    assert_eq!(code, StatusCode::OK);
    let endpoint = format!("/api/projects/{}/reviews", project["id"]);
    for (side, expected) in [
        ("head", "committed"),
        ("index", "staged"),
        ("worktree", "working"),
    ] {
        let (code, saved) = send_json(
            &app,
            "POST",
            &endpoint,
            json!({"mode":"codebase", "head":"feature/test", "side":side}),
        )
        .await;
        assert_eq!(code, StatusCode::OK, "{saved}");
        assert_eq!(saved["scopeMode"], "codebase");
        assert_eq!(saved["baseRef"], "");
        let id = saved["id"].as_i64().unwrap();
        let (code, loaded) =
            send_json(&app, "GET", &format!("/api/reviews/{id}"), json!(null)).await;
        assert_eq!(code, StatusCode::OK);
        assert_eq!(loaded["scopeMode"], "codebase");
        let (code, diff) =
            send_json(&app, "GET", &format!("/api/reviews/{id}/diff"), json!(null)).await;
        assert_eq!(code, StatusCode::OK, "{diff}");
        let files = diff["files"].as_array().unwrap();
        let unchanged = files
            .iter()
            .find(|file| file["newPath"] == "image.png")
            .unwrap();
        assert_eq!(unchanged["binary"], true);
        let text = files
            .iter()
            .find(|file| file["newPath"] == "a.txt")
            .unwrap();
        assert_eq!(text["hunks"][0]["lines"][0]["content"], expected);
        assert_eq!(text["hunks"][0]["lines"][0]["newLine"], 1);
        assert_eq!(
            files.iter().any(|file| file["newPath"] == "untracked.txt"),
            side == "worktree"
        );
        let (code, comment) = send_json(&app, "POST", &format!("/api/reviews/{id}/comments"), json!({"filePath":"a.txt", "startLine":1,"endLine":1,"side":side,"body":"Codebase finding"})).await;
        assert_eq!(code, StatusCode::OK, "{comment}");
        assert_eq!(comment["snippet"], expected);
        let (code, _) = send_json(
            &app,
            "POST",
            &format!("/api/reviews/{id}/reviewed"),
            json!({"filePaths":["a.txt"], "reviewed":true,"side":side}),
        )
        .await;
        assert_eq!(code, StatusCode::NO_CONTENT);
        let (code, _) = send_json(&app, "POST", &format!("/api/reviews/{id}/comments"), json!({"filePath":"a.txt", "side":if side == "head" { "index" } else { "head" },"body":"Wrong source"})).await;
        assert_eq!(code, StatusCode::BAD_REQUEST);
    }
    for body in [
        json!({"mode":"other", "head":"main", "side":"head"}),
        json!({"mode":"codebase", "head":"main", "base":"main", "side":"head"}),
        json!({"mode":"codebase", "head":"main", "from":"main", "side":"head"}),
        json!({"mode":"codebase", "head":"missing", "side":"head"}),
        json!({"mode":"codebase", "head":"main", "side":"index"}),
    ] {
        let (code, error) = send_json(&app, "POST", &endpoint, body).await;
        assert_eq!(code, StatusCode::BAD_REQUEST, "{error}");
    }
    // Codebase mode has no dependency on main/master or a merge target.
    f.git(&["branch", "-m", "main", "trunk"]);
    let (code, review) = send_json(
        &app,
        "POST",
        &endpoint,
        json!({"mode":"codebase", "head":"trunk", "side":"head"}),
    )
    .await;
    assert_eq!(code, StatusCode::OK, "{review}");
    let (code, diff) = send_json(
        &app,
        "GET",
        &format!("/api/reviews/{}/diff", review["id"]),
        json!(null),
    )
    .await;
    assert_eq!(code, StatusCode::OK, "{diff}");
    assert!(
        diff["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|file| file["newPath"] == "a.txt")
    );
}

#[tokio::test]
async fn whole_folder_reviews_save_and_open_with_detached_or_unborn_head() {
    use serde_json::json;
    let f = Fixture::new();
    let app = f.app();
    let (_, project) = send_json(&app, "POST", "/api/projects", json!({"path":f.0})).await;
    let endpoint = format!("/api/projects/{}/reviews", project["id"]);
    f.git(&["checkout", "--detach"]);
    for unborn in [false, true] {
        if unborn {
            f.git(&["symbolic-ref", "HEAD", "refs/heads/unborn"]);
        }
        let (code, saved) = send_json(
            &app,
            "POST",
            &endpoint,
            json!({"mode":"codebase", "head":"HEAD", "side":"worktree"}),
        )
        .await;
        assert_eq!(code, StatusCode::OK, "{saved}");
        let id = saved["id"].as_i64().unwrap();
        let (code, diff) =
            send_json(&app, "GET", &format!("/api/reviews/{id}/diff"), json!(null)).await;
        assert_eq!(code, StatusCode::OK, "{diff}");
        let files = diff["files"].as_array().unwrap();
        assert!(files.iter().any(|file| file["newPath"] == "a.txt"));
        assert!(files.iter().any(|file| file["newPath"] == "untracked.txt"));
        let (code, comment) = send_json(&app, "POST", &format!("/api/reviews/{id}/comments"), json!({"filePath":"a.txt","startLine":1,"endLine":1,"side":"worktree","body":"Folder finding"})).await;
        assert_eq!(code, StatusCode::OK, "{comment}");
        assert_eq!(comment["snippet"], "working");
        let (code, _) = send_json(
            &app,
            "POST",
            &format!("/api/reviews/{id}/reviewed"),
            json!({"filePaths":["a.txt"],"reviewed":true,"side":"worktree"}),
        )
        .await;
        assert_eq!(code, StatusCode::NO_CONTENT);
        let (code, loaded) =
            send_json(&app, "GET", &format!("/api/reviews/{id}"), json!(null)).await;
        assert_eq!(code, StatusCode::OK);
        assert!(loaded.get("annotationError").is_none(), "{loaded}");
        assert_eq!(loaded["reviewedFiles"], json!(["a.txt"]));
        assert_eq!(loaded["comments"][0]["anchorStatus"], "current");
    }
}
