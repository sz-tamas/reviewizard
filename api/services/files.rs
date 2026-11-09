//! File-side selection, content fallback and binary reads shared by file/blob routes.
use crate::{
    services::{git::Repo, workspace::Workspace},
    utils::{
        errors::{ErrorKind, ServiceError},
        validation::{valid_ref, validate_path},
    },
};
use serde::{Deserialize, Serialize};
#[derive(Default, Deserialize)]
#[serde(default)]
pub struct FileQuery {
    repo: String,
    #[serde(rename = "ref")]
    reference: String,
    path: String,
    side: String,
}
#[derive(Serialize)]
pub struct Files {
    files: Option<Vec<String>>,
}
pub async fn list(workspace: &Workspace, q: FileQuery) -> Result<Files, ServiceError> {
    let repo = workspace.repo(&q.repo)?;
    valid_ref(&q.reference)?;
    let bytes = repo
        .run(&[
            "-c",
            "core.quotePath=false",
            "ls-tree",
            "-r",
            "--name-only",
            &q.reference,
        ])
        .await?;
    let files: Vec<String> = String::from_utf8_lossy(&bytes)
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect();
    Ok(Files {
        files: if files.is_empty() { None } else { Some(files) },
    })
}
#[derive(Serialize)]
pub struct File {
    path: String,
    #[serde(rename = "ref")]
    reference: String,
    content: String,
    worktree: bool,
}
async fn read(repo: Repo, q: &FileQuery) -> Result<(Vec<u8>, bool), ServiceError> {
    validate_path(&q.path)?;
    match q.side.as_str() {
        "worktree" => Ok((
            super::review::read_side(&repo, &q.reference, &q.path, super::store::Side::Worktree)
                .await?,
            true,
        )),
        "index" => Ok((
            super::review::read_side(&repo, &q.reference, &q.path, super::store::Side::Index)
                .await?,
            false,
        )),
        "" | "head" => {
            valid_ref(&q.reference)?;
            match repo.object(&format!("{}:{}", q.reference, q.path)).await {
                Ok(bytes) => Ok((bytes, false)),
                Err(e) if e.kind == ErrorKind::NotFound => {
                    match repo.worktree_file(&q.path).await {
                        Ok(bytes) => Ok((bytes, true)),
                        Err(_) => Err(e),
                    }
                }
                Err(e) => Err(e),
            }
        }
        _ => Err(ServiceError::bad(
            "invalid side: want \"head\", \"worktree\" or \"index\"",
        )),
    }
}
async fn read_with_context(repo: Repo, q: &FileQuery) -> Result<(Vec<u8>, bool), ServiceError> {
    read(repo, q).await.map_err(|error| {
        if error.kind == ErrorKind::NotFound {
            let side = super::store::Side::parse(&q.side).unwrap_or_default();
            ServiceError::missing(format!(
                "{} does not exist in {}",
                q.path,
                super::review::side_label(side, &q.reference)
            ))
        } else {
            error
        }
    })
}
pub async fn file(workspace: &Workspace, q: FileQuery) -> Result<File, ServiceError> {
    let repo = workspace.repo(&q.repo)?;
    let (content, worktree) = read_with_context(repo, &q).await?;
    Ok(File {
        path: q.path,
        reference: q.reference,
        content: String::from_utf8_lossy(&content).into_owned(),
        worktree,
    })
}

pub struct Blob {
    pub content: Vec<u8>,
    pub mime: String,
}
pub async fn blob(workspace: &Workspace, q: FileQuery) -> Result<Blob, ServiceError> {
    let repo = workspace.repo(&q.repo)?;
    let (content, _) = read_with_context(repo, &q).await?;
    Ok(Blob {
        content,
        mime: crate::utils::mime::for_path(&q.path),
    })
}
