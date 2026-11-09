//! Bounded argument-array Git reads. No shell interpolation.
use crate::utils::{errors::ServiceError, process, validation::validate_path};
use std::path::PathBuf;
#[derive(Debug)]
pub struct Repo {
    pub path: PathBuf,
}
pub mod attributes;
pub mod batch;
pub mod diff;
pub mod fingerprint;
pub mod linemap;
pub mod refs;
impl Repo {
    pub async fn run(&self, args: &[&str]) -> Result<Vec<u8>, ServiceError> {
        process::git(&self.path, args).await
    }
    pub async fn object(&self, spec: &str) -> Result<Vec<u8>, ServiceError> {
        match self.run(&["show", spec]).await {
            Ok(bytes) => Ok(bytes),
            Err(e) => {
                if self.run(&["cat-file", "-e", spec]).await.is_err() {
                    Err(ServiceError::missing("not found"))
                } else {
                    Err(e)
                }
            }
        }
    }
    pub async fn worktree_file(&self, path: &str) -> Result<Vec<u8>, ServiceError> {
        let resolved = self.worktree_path(path).await?;
        tokio::fs::read(resolved).await.map_err(ServiceError::from)
    }
    pub async fn worktree_path(&self, path: &str) -> Result<PathBuf, ServiceError> {
        validate_path(path)?;
        let resolved = tokio::fs::canonicalize(self.path.join(path))
            .await
            .map_err(ServiceError::from)?;
        let root = tokio::fs::canonicalize(&self.path)
            .await
            .map_err(ServiceError::from)?;
        let rel = resolved
            .strip_prefix(&root)
            .map_err(|_| ServiceError::bad("invalid path"))?;
        validate_path(&rel.to_string_lossy())?;
        if !tokio::fs::metadata(&resolved)
            .await
            .map_err(ServiceError::from)?
            .is_file()
        {
            return Err(ServiceError::bad("path is not a regular file"));
        }
        Ok(resolved)
    }
    pub async fn main_branch(&self) -> String {
        let candidates = [
            "refs/heads/main",
            "refs/heads/master",
            "refs/remotes/origin/HEAD",
            "refs/remotes/origin/main",
            "refs/remotes/origin/master",
        ];
        let mut args = vec!["for-each-ref", "--format=%(refname)\x1f%(symref:short)"];
        args.extend(candidates);
        let Ok(bytes) = self.run(&args).await else {
            return String::new();
        };
        let text = String::from_utf8_lossy(&bytes);
        let found: std::collections::HashMap<_, _> = text
            .lines()
            .filter_map(|line| line.split_once('\x1f'))
            .collect();
        for name in candidates {
            if let Some(target) = found.get(name) {
                if name == "refs/remotes/origin/HEAD" {
                    if !target.trim().is_empty() && *target != "origin/HEAD" {
                        return target.trim().into();
                    }
                } else {
                    return name
                        .trim_start_matches("refs/heads/")
                        .trim_start_matches("refs/remotes/")
                        .into();
                }
            }
        }
        String::new()
    }
}
