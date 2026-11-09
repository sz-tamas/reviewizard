//! Repository discovery and root confinement.
use crate::{services::git::Repo, utils::errors::ServiceError};
use chrono::{DateTime, Local};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    io,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};
#[derive(Debug, Clone)]
pub struct Workspace {
    pub root: PathBuf,
    projects: Arc<RwLock<BTreeMap<i64, crate::services::store::Project>>>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    name: String,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    display_name: Option<String>,
    path: PathBuf,
    last_activity: String,
}
impl Workspace {
    pub fn open(root: &Path) -> io::Result<Self> {
        let root = crate::utils::paths::absolute_clean(root)?;
        if !root.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "root is not a directory",
            ));
        }
        Ok(Self {
            root,
            projects: Default::default(),
        })
    }
    pub fn register(&self, project: crate::services::store::Project) -> Result<(), ServiceError> {
        self.projects
            .write()
            .map_err(ServiceError::internal)?
            .insert(project.id, project);
        Ok(())
    }
    pub fn reload_projects(
        &self,
        store: &crate::services::store::Store,
    ) -> Result<(), ServiceError> {
        let mut registry = self.projects.write().map_err(ServiceError::internal)?;
        *registry = store.projects()?.into_iter().map(|p| (p.id, p)).collect();
        Ok(())
    }
    fn single_name(&self) -> Option<String> {
        self.root.join(".git").exists().then(|| {
            self.root
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        })
    }
    pub fn repo(&self, name: &str) -> Result<Repo, ServiceError> {
        if let Some(id) = name.strip_prefix("@project/") {
            let id: i64 = id
                .parse()
                .map_err(|_| ServiceError::bad("invalid project ID"))?;
            let projects = self.projects.read().map_err(ServiceError::internal)?;
            let project = projects
                .get(&id)
                .ok_or_else(|| ServiceError::missing("project not found"))?;
            let path = PathBuf::from(&project.path);
            let canonical = path.canonicalize().map_err(ServiceError::from)?;
            if canonical != Path::new(&project.canonical_path) {
                return Err(ServiceError::bad("project folder identity has changed"));
            }
            if !path.join(".git").exists() {
                return Err(ServiceError::bad("project is no longer a Git repository"));
            }
            return Ok(Repo { path });
        }
        if name.is_empty() {
            return Err(ServiceError::bad("repo is required"));
        }
        if let Some(single) = self.single_name() {
            return if name == single {
                Ok(Repo {
                    path: self.root.clone(),
                })
            } else {
                Err(ServiceError::bad("invalid repo name"))
            };
        }
        if name == "." || name == ".." || name.contains(['/', '\\', '\0']) {
            return Err(ServiceError::bad("invalid repo name"));
        }
        let path = self.root.join(name);
        if !path.join(".git").exists() {
            return Err(ServiceError::bad(format!("not a git repository: {name}")));
        }
        let path = path
            .canonicalize()
            .map_err(|_| ServiceError::bad("invalid repo name"))?;
        let canonical_root = self
            .root
            .canonicalize()
            .map_err(|_| ServiceError::bad("invalid repo name"))?;
        if !path.starts_with(&canonical_root) {
            return Err(ServiceError::bad("invalid repo name"));
        }
        Ok(Repo {
            path: self.root.join(name),
        })
    }
    pub fn list(&self) -> Result<Vec<Entry>, ServiceError> {
        let projects = self.projects.read().map_err(ServiceError::internal)?;
        let mut entries: Vec<Entry> = projects
            .values()
            .filter(|p| !p.archived)
            .map(|p| Entry {
                name: format!("@project/{}", p.id),
                display_name: Some(p.name.clone()),
                path: PathBuf::from(&p.path),
                last_activity: activity(Path::new(&p.path)),
            })
            .collect();
        for entry in self.startup_list()? {
            if !entries.iter().any(|p| p.path == entry.path) {
                entries.push(entry);
            }
        }
        Ok(entries)
    }
    fn startup_list(&self) -> Result<Vec<Entry>, ServiceError> {
        if let Some(name) = self.single_name() {
            return Ok(vec![Entry {
                name,
                display_name: None,
                path: self.root.clone(),
                last_activity: activity(&self.root),
            }]);
        }
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(&self.root).map_err(ServiceError::internal)? {
            let entry = entry.map_err(ServiceError::internal)?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if !entry.file_type().map_err(ServiceError::internal)?.is_dir() || name.starts_with('.')
            {
                continue;
            }
            if entry.path().join(".git").exists() {
                entries.push(Entry {
                    name,
                    display_name: None,
                    path: entry.path(),
                    last_activity: activity(&entry.path()),
                });
            }
        }
        entries.sort_by(|a, b| {
            b.last_activity
                .cmp(&a.last_activity)
                .then_with(|| a.name.cmp(&b.name))
        });
        Ok(entries)
    }
}
fn activity(path: &Path) -> String {
    for candidate in [path.join(".git/logs/HEAD"), path.join(".git")] {
        if let Ok(time) = candidate.metadata().and_then(|m| m.modified()) {
            let local: DateTime<Local> = time.into();
            return local.format("%Y-%m-%d").to_string();
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::{
        fs::{File, FileTimes},
        sync::atomic::{AtomicUsize, Ordering},
    };
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!(
                "workspace-regression-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn repo(root: &Path, name: &str, day: u32, hour: u32) {
        let path = root.join(name).join(".git/logs");
        std::fs::create_dir_all(&path).unwrap();
        let file = File::create(path.join("HEAD")).unwrap();
        let time: std::time::SystemTime = Local
            .with_ymd_and_hms(2024, 3, day, hour, 0, 0)
            .unwrap()
            .into();
        file.set_times(FileTimes::new().set_modified(time)).unwrap();
    }
    #[test]
    fn regression_repository_order_single_root_and_traversal() {
        let f = Fixture::new();
        repo(&f.0, "delta", 4, 17);
        repo(&f.0, "beta", 4, 9);
        repo(&f.0, "zulu", 3, 9);
        repo(&f.0, "alpha", 1, 9);
        std::fs::create_dir(f.0.join("notarepo")).unwrap();
        std::fs::write(f.0.join("loose.txt"), "x").unwrap();
        let w = Workspace::open(&f.0).unwrap();
        let entries = w.list().unwrap();
        assert_eq!(
            entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
            ["beta", "delta", "zulu", "alpha"]
        );
        assert!(entries.iter().all(|e| e.last_activity.len() == 10));
        for entry in &entries {
            assert!(entry.path.is_absolute());
            assert_eq!(entry.path, w.repo(&entry.name).unwrap().path);
        }
        assert!(w.repo("beta").is_ok());
        for value in ["", ".", "..", "../beta", "a/b", "a\\b", "nope"] {
            assert!(w.repo(value).is_err(), "{value}");
        }
        repo(&f.0.join("beta"), "vendored", 4, 9);
        let w = Workspace::open(&f.0.join("beta")).unwrap();
        let entries = w.list().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, f.0.join("beta"));
        assert!(w.repo("beta").is_ok());
        for value in ["", ".", "..", "vendored", "beta/vendored"] {
            assert!(w.repo(value).is_err());
        }
    }
    #[test]
    fn regression_gitlink_activity_date() {
        let f = Fixture::new();
        let path = f.0.join("linked");
        std::fs::create_dir(&path).unwrap();
        let file = File::create(path.join(".git")).unwrap();
        let time: std::time::SystemTime =
            Local.with_ymd_and_hms(2024, 3, 4, 12, 0, 0).unwrap().into();
        file.set_times(FileTimes::new().set_modified(time)).unwrap();
        assert_eq!(activity(&path), "2024-03-04");
    }
}
