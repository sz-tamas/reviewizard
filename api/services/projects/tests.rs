use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "reviewizard-projects-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn repo(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::create_dir_all(&path).unwrap();
        let out = std::process::Command::new("git")
            .args(["init", "-b", "main"])
            .arg(&path)
            .output()
            .unwrap();
        assert!(out.status.success());
        path
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[tokio::test]
async fn independent_same_named_projects_persist_and_removal_preserves_history() {
    let f = Fixture::new();
    let a = f.repo("a/repo");
    let b = f.repo("b/repo");
    let db = f.0.join("reviews.db");
    let store = Store::open(&db).unwrap();
    let workspace = Workspace::open(&f.0).unwrap();
    let first = add(
        &store,
        &workspace,
        AddProject {
            path: a.to_str().unwrap().into(),
        },
    )
    .await
    .unwrap();
    let second = add(
        &store,
        &workspace,
        AddProject {
            path: b.to_str().unwrap().into(),
        },
    )
    .await
    .unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(first.name, second.name);
    let r = store
        .create_or_get_review(&first.path, "main", "feature", "sha")
        .unwrap();
    store
        .create_or_get_review(&second.path, "main", "feature", "sha")
        .unwrap();
    assert_eq!(
        workspace
            .repo(&format!("@project/{}", first.id))
            .unwrap()
            .path,
        a
    );
    assert_eq!(
        workspace
            .repo(&format!("@project/{}", second.id))
            .unwrap()
            .path,
        b
    );
    remove(&store, &workspace, first.id).await.unwrap();
    assert_eq!(list(&store).await.unwrap().len(), 1);
    assert_eq!(store.get_review(r.id).unwrap().repo_path, first.path);
    assert_eq!(
        workspace
            .repo(&format!("@project/{}", first.id))
            .unwrap()
            .path,
        a
    );
    let restored = add(
        &store,
        &workspace,
        AddProject {
            path: first.path.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(restored.id, first.id);
    assert_eq!(restored.review_count, 1);
    drop(store);
    let reopened = Store::open(&db).unwrap();
    let fresh_workspace = Workspace::open(&f.0).unwrap();
    for p in reopened.projects().unwrap() {
        fresh_workspace.register(p).unwrap();
    }
    assert_eq!(list(&reopened).await.unwrap().len(), 2);
    assert_eq!(
        fresh_workspace
            .repo(&format!("@project/{}", first.id))
            .unwrap()
            .path,
        a
    );
    assert_eq!(reopened.get_review(r.id).unwrap().repo_path, first.path);
}

#[tokio::test]
async fn invalid_paths_never_register_and_discovery_is_confined() {
    let f = Fixture::new();
    let store = Store::in_memory().unwrap();
    let workspace = Workspace::open(&f.0).unwrap();
    for path in [
        "relative".to_owned(),
        "".into(),
        f.0.to_str().unwrap().into(),
    ] {
        assert!(add(&store, &workspace, AddProject { path }).await.is_err());
    }
    assert!(store.projects().unwrap().is_empty());
    assert!(remove(&store, &workspace, 100).await.is_err());
    assert!(workspace.repo("@project/999").is_err());
    assert!(browse(&f.0, None, f.0.parent().unwrap().to_str().unwrap()).is_err());
    let repo = f.repo("repo");
    let listing = browse(&f.0, None, "").unwrap();
    assert!(listing.parent.is_none());
    assert!(
        listing
            .folders
            .iter()
            .any(|p| p.is_repository && p.path == repo.canonicalize().unwrap())
    );
    assert!(!listing.folders.iter().any(|p| p.name == ".git"));
    let file = f.0.join("regular-file");
    std::fs::write(&file, "not a directory").unwrap();
    assert_eq!(
        browse(&f.0, None, file.to_str().unwrap())
            .err()
            .unwrap()
            .kind,
        crate::utils::errors::ErrorKind::BadRequest
    );
}

#[cfg(unix)]
#[tokio::test]
async fn replacing_registered_symlink_cannot_redirect_review_identity() {
    let f = Fixture::new();
    let a = f.repo("a");
    let b = f.repo("b");
    let alias = f.0.join("alias");
    std::os::unix::fs::symlink(&a, &alias).unwrap();
    let store = Store::in_memory().unwrap();
    let workspace = Workspace::open(&f.0).unwrap();
    let project = add(
        &store,
        &workspace,
        AddProject {
            path: alias.to_str().unwrap().into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        workspace
            .repo(&format!("@project/{}", project.id))
            .unwrap()
            .path,
        alias
    );
    std::fs::remove_file(&alias).unwrap();
    std::os::unix::fs::symlink(&b, &alias).unwrap();
    assert!(workspace.repo(&format!("@project/{}", project.id)).is_err());
    assert!(
        add(&store, &workspace, AddProject { path: project.path })
            .await
            .is_err()
    );
    assert!(
        browse(&f.0, None, "")
            .unwrap()
            .folders
            .iter()
            .all(|p| p.name != "alias")
    );
}
