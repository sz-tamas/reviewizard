//! Configuration, persistence and HTTP regressions.
use super::*;
use clap::Parser;
use reviewizard::services::{config::Config, transition};
#[test]
fn canonical_cli_flags() {
    let c = Config::try_parse_from([
        "app",
        "--root",
        "-folder",
        "--port=0",
        "--retention-days",
        "-1",
    ])
    .unwrap();
    assert_eq!(c.root, std::path::Path::new("-folder"));
    assert_eq!(c.port, 0);
    let c = Config::try_parse_from(["app"]).unwrap();
    assert_eq!(c.port, 7777);
}
#[test]
fn transition_backup_includes_wal_and_is_idempotent() {
    let f = Fixture::new();
    let data = f.0.join("data");
    let store = Store::open_directory(&data).unwrap();
    let review = store
        .create_or_get_review(f.0.to_str().unwrap(), "main", "feature/test", "sha")
        .unwrap();
    transition::prepare(&data).unwrap();
    let backup = data.join("reviewizard.before-rust.db");
    let original = std::fs::read(&backup).unwrap();
    let copy =
        rusqlite::Connection::open_with_flags(&backup, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let path: String = copy
        .query_row(
            "SELECT repo_path FROM reviews WHERE id=?",
            [review.id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(path, f.0.to_str().unwrap());
    drop(copy);
    store.set_review_summary(review.id, "after backup").unwrap();
    transition::prepare(&data).unwrap();
    assert!(
        std::fs::read(&backup).unwrap() == original,
        "the original backup must never be overwritten"
    );
    assert!(data.join("rust-transition.json").exists());
}
#[test]
fn existing_backup_requires_recovery_and_dual_data_requires_selection() {
    let f = Fixture::new();
    let home = f.0.join("home");
    let data = home.join(".reviewizard");
    Store::open_directory(&data).unwrap();
    std::fs::write(data.join("reviewizard.before-rust.db"), "preserve me").unwrap();
    assert!(transition::prepare(&data).is_err());
    let prior = home.join(".reviewizard-rust");
    Store::open_directory(&prior).unwrap();
    assert!(reviewizard::services::application::resolve_data_dir(None, Some(&home)).is_err());
    assert_eq!(
        reviewizard::services::application::resolve_data_dir(Some(&data), Some(&home)).unwrap(),
        data
    );
}
#[tokio::test]
async fn raw_mutation_json_compatibility_and_strict_trailing_document() {
    let f = Fixture::new();
    let app = f.app();
    for (body, mime, status) in [
        (
            format!(
                r#"{{"Repo":"{}","Head":"feature/test","Base":null}}"#,
                f.repo()
            ),
            None,
            200,
        ),
        (
            format!(
                r#"{{"repo":"{}","head":"feature/test","base":"main"}}"#,
                f.repo()
            ),
            Some("text/plain"),
            200,
        ),
        (
            format!(
                r#"{{"repo":"{}","head":"feature/test","base":"main"}} {{}}"#,
                f.repo()
            ),
            Some("application/json"),
            400,
        ),
    ] {
        let mut request = Request::builder().method("POST").uri("/api/reviews");
        if let Some(mime) = mime {
            request = request.header("content-type", mime);
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::from(body)).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), status);
    }
}
#[tokio::test]
async fn backend_root_and_api_method_fallback() {
    let router = app();
    let response = router
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/repos")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(response.headers()["content-type"], "application/json");
    assert!(response.headers().get("allow").is_none());
}
#[cfg(unix)]
#[test]
fn aliases_preserve_logical_identity_and_canonical_confinement() {
    let f = Fixture::new();
    let holder = f.0.join("holder");
    std::fs::create_dir(&holder).unwrap();
    let alias = holder.join("alias");
    std::os::unix::fs::symlink(&f.0, &alias).unwrap();
    let w = Workspace::open(&alias).unwrap();
    assert_eq!(w.repo("alias").unwrap().path, alias);
    assert!(w.repo(f.repo()).is_err());
    let parent = Workspace::open(&holder).unwrap();
    assert!(parent.repo("alias").is_err());
}
