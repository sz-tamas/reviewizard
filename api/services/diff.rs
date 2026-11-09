//! Resolve comparison semantics once, then obtain and annotate the structured diff.
use crate::{
    services::{
        git::{
            Repo, attributes,
            diff::{self, FileDiff},
            refs,
        },
        workspace::Workspace,
    },
    utils::{errors::ServiceError, validation::valid_ref},
};
use serde::{Deserialize, Serialize};
#[derive(Default, Deserialize)]
#[serde(default)]
pub struct DiffQuery {
    repo: String,
    head: String,
    base: String,
    from: String,
    uncommitted: String,
    unstaged: String,
}
#[derive(Serialize)]
pub struct Diff {
    pub base: String,
    pub head: String,
    pub files: Option<Vec<FileDiff>>,
}
fn unrelated(base: &str, head: &str) -> ServiceError {
    ServiceError::bad(format!(
        "{head} and {base} share no common history — pick a base branch the work was started from"
    ))
}
pub(crate) async fn comparison_base(
    repo: &Repo,
    base: &str,
    head: &str,
) -> Result<(String, String), ServiceError> {
    if !base.is_empty() {
        match refs::merge_base(repo, base, head).await {
            Ok(Some(sha)) => return Ok((base.into(), sha)),
            Ok(None) => return Err(unrelated(base, head)),
            Err(_) => {} // Fall back to the default branch for a stale base.
        }
    }
    let main = repo.main_branch().await;
    if main.is_empty() {
        return Err(ServiceError::bad(
            "no main or master branch found; select a base branch",
        ));
    }
    refs::merge_base(repo, &main, head)
        .await?
        .map(|sha| (main.clone(), sha))
        .ok_or_else(|| unrelated(&main, head))
}
pub async fn get(workspace: &Workspace, q: DiffQuery) -> Result<Diff, ServiceError> {
    let repo = workspace.repo(&q.repo)?;
    valid_ref(&q.head)?;
    let base = if q.from.is_empty() || q.from == "all" {
        if !q.base.is_empty() {
            valid_ref(&q.base)?;
        }
        comparison_base(&repo, &q.base, &q.head).await?.1
    } else {
        valid_ref(&q.from)?;
        refs::parent(&repo, &q.from)
            .await
            .map_err(|_| ServiceError::bad(format!("unknown commit: {}", q.from)))?
    };
    let mut files = diff::read(
        &repo,
        &base,
        &q.head,
        q.uncommitted == "true",
        q.unstaged != "false",
    )
    .await?;
    attributes::mark(&repo, &mut files).await;
    Ok(Diff {
        base,
        head: q.head,
        files: if files.is_empty() { None } else { Some(files) },
    })
}
