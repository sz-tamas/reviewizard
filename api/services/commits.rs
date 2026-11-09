//! Commit history and base/limit selection; independent of HTTP.
use crate::{
    services::workspace::Workspace,
    utils::{errors::ServiceError, validation::valid_ref},
};
use serde::{Deserialize, Serialize};
#[derive(Default, Deserialize)]
#[serde(default)]
pub struct CommitQuery {
    repo: String,
    #[serde(rename = "ref")]
    reference: String,
    base: String,
    limit: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Commit {
    sha: String,
    short_sha: String,
    subject: String,
    rel_date: String,
}
#[derive(Serialize)]
pub struct Commits {
    commits: Option<Vec<Commit>>,
}
pub async fn list(workspace: &Workspace, q: CommitQuery) -> Result<Commits, ServiceError> {
    let repo = workspace.repo(&q.repo)?;
    valid_ref(&q.reference)?;
    if !q.base.is_empty() {
        valid_ref(&q.base)?;
    }
    let limit = q
        .limit
        .parse::<usize>()
        .ok()
        .filter(|n| *n > 0)
        .unwrap_or(50)
        .min(200)
        .to_string();
    let base = if !q.base.is_empty()
        && repo
            .run(&["rev-parse", "--verify", &format!("{}^{{commit}}", q.base)])
            .await
            .is_ok()
    {
        q.base
    } else {
        repo.main_branch().await
    };
    let range = if base.is_empty() {
        q.reference
    } else {
        format!("{}..{}", base, q.reference)
    };
    let bytes = repo
        .run(&[
            "log",
            &range,
            "-n",
            &limit,
            "--format=%H%x1f%h%x1f%s%x1f%cr",
        ])
        .await?;
    let mut commits = Vec::new();
    for line in String::from_utf8_lossy(&bytes).lines() {
        let fields: Vec<_> = line.split('\x1f').collect();
        if let [sha, short_sha, subject, rel_date] = fields.as_slice() {
            commits.push(Commit {
                sha: sha.to_string(),
                short_sha: short_sha.to_string(),
                subject: subject.to_string(),
                rel_date: rel_date.to_string(),
            });
        }
    }
    Ok(Commits {
        commits: if commits.is_empty() {
            None
        } else {
            Some(commits)
        },
    })
}
