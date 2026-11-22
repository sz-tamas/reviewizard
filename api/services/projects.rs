//! Explicit local project registration and confined directory discovery.
use crate::{
    services::{
        git::Repo,
        store::{Project, Store},
        workspace::Workspace,
    },
    utils::{errors::ServiceError, paths},
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

mod metadata;

#[derive(Deserialize)]
pub struct AddProject {
    pub path: String,
}

#[derive(Serialize)]
pub struct ListedProject {
    #[serde(flatten)]
    project: Project,
    metadata: Option<metadata::ProjectMetadata>,
}

pub async fn list(store: &Store) -> Result<Vec<ListedProject>, ServiceError> {
    use futures_util::{StreamExt, stream};
    let projects = store.run(|s| s.projects()).await?;
    Ok(stream::iter(projects.into_iter().filter(|p| !p.archived))
        .map(|project| async move {
            let metadata = metadata::collect(&project).await;
            ListedProject { project, metadata }
        })
        .buffered(4)
        .collect()
        .await)
}

pub async fn add(
    store: &Store,
    workspace: &Workspace,
    input: AddProject,
) -> Result<Project, ServiceError> {
    if input.path.trim().is_empty()
        || input.path.contains('\0')
        || input.path.len() > 4096
        || !Path::new(&input.path).is_absolute()
    {
        return Err(ServiceError::bad(
            "Choose an absolute local repository folder",
        ));
    }
    let path = paths::absolute_clean(Path::new(&input.path)).map_err(ServiceError::from)?;
    let canonical = tokio::fs::canonicalize(&path)
        .await
        .map_err(ServiceError::from)?;
    if !tokio::fs::metadata(&canonical)
        .await
        .map_err(ServiceError::from)?
        .is_dir()
        || !tokio::fs::try_exists(path.join(".git"))
            .await
            .map_err(ServiceError::from)?
    {
        return Err(ServiceError::bad(
            "Choose a Git repository root containing .git",
        ));
    }
    let repo = Repo { path: path.clone() };
    let top = repo.run(&["rev-parse", "--show-toplevel"]).await?;
    let top = String::from_utf8(top).map_err(ServiceError::internal)?;
    if tokio::fs::canonicalize(top.strip_suffix('\n').unwrap_or(&top))
        .await
        .map_err(ServiceError::from)?
        != canonical
    {
        return Err(ServiceError::bad(
            "Choose the repository root, rather than a subfolder",
        ));
    }
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let logical = path
        .to_str()
        .ok_or_else(|| ServiceError::bad("Folder path must be UTF-8"))?
        .to_owned();
    let canonical = canonical
        .to_str()
        .ok_or_else(|| ServiceError::bad("Folder path must be UTF-8"))?
        .to_owned();
    let project = store
        .run(move |s| s.add_project(&name, &logical, &canonical))
        .await?;
    let workspace = workspace.clone();
    store.run(move |s| workspace.reload_projects(s)).await?;
    Ok(project)
}

pub async fn remove(store: &Store, workspace: &Workspace, id: i64) -> Result<(), ServiceError> {
    // Keep the registered identity available to existing review requests; hide only the project card.
    store
        .run(move |s| {
            s.archive_project(id)?;
            Ok(())
        })
        .await?;
    let workspace = workspace.clone();
    store.run(move |s| workspace.reload_projects(s)).await
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct FolderQuery {
    pub path: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub name: String,
    pub path: PathBuf,
    pub is_repository: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Folders {
    pub path: PathBuf,
    pub parent: Option<PathBuf>,
    pub roots: Vec<PathBuf>,
    pub folders: Vec<Folder>,
    pub is_repository: bool,
}

pub async fn folders(workspace: Workspace, query: FolderQuery) -> Result<Folders, ServiceError> {
    tokio::task::spawn_blocking(move || {
        browse(&workspace.root, paths::home().as_deref(), &query.path)
    })
    .await
    .map_err(ServiceError::internal)?
}

fn browse(startup: &Path, home: Option<&Path>, requested: &str) -> Result<Folders, ServiceError> {
    let mut roots = vec![startup.canonicalize().map_err(ServiceError::from)?];
    if let Some(home) = home {
        let home = home.canonicalize().map_err(ServiceError::from)?;
        if !roots.contains(&home) {
            roots.push(home);
        }
    }
    let path = if requested.is_empty() {
        roots.last().unwrap().clone()
    } else {
        if !Path::new(requested).is_absolute() || requested.len() > 4096 || requested.contains('\0')
        {
            return Err(ServiceError::bad("invalid folder path"));
        }
        PathBuf::from(requested)
            .canonicalize()
            .map_err(ServiceError::from)?
    };
    let allowed = |p: &Path| roots.iter().any(|root| p.starts_with(root));
    if !allowed(&path) {
        return Err(ServiceError::bad(
            "Folder browsing is confined to the home and startup folders; enter another repository's absolute path to add it explicitly",
        ));
    }
    if !std::fs::metadata(&path)
        .map_err(ServiceError::from)?
        .is_dir()
    {
        return Err(ServiceError::bad("Choose a directory to browse"));
    }
    let mut folders = Vec::new();
    for (i, entry) in std::fs::read_dir(&path)
        .map_err(ServiceError::from)?
        .enumerate()
    {
        if i >= 4096 {
            return Err(ServiceError::bad(
                "Too many folders to browse; enter the repository path directly",
            ));
        }
        let entry = entry.map_err(ServiceError::from)?;
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        // Do not follow symlink entries: discovery cannot escape its roots.
        if !entry.file_type().map_err(ServiceError::from)?.is_dir() {
            continue;
        }
        folders.push(Folder {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: entry.path(),
            is_repository: entry.path().join(".git").exists(),
        });
    }
    folders.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(Folders {
        parent: path.parent().filter(|p| allowed(p)).map(Path::to_path_buf),
        is_repository: path.join(".git").exists(),
        path,
        roots,
        folders,
    })
}

#[cfg(test)]
mod tests;
