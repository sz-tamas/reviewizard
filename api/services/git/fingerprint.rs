//! Content-free change signals with bounded polling cadence and optional-lock protection.
use super::Repo;
use crate::utils::{errors::ServiceError, process};
use sha2::{Digest, Sha256};
use std::time::UNIX_EPOCH;
async fn run(repo: &Repo, args: &[&str]) -> Result<Vec<u8>, ServiceError> {
    process::checked(
        process::git_output_with_env(&repo.path, args, None, &[("GIT_OPTIONAL_LOCKS", "0")])
            .await?,
        args,
    )
}
pub async fn refs(repo: &Repo) -> Result<String, ServiceError> {
    let head = run(repo, &["rev-parse", "HEAD", "--symbolic-full-name", "HEAD"]).await?;
    let refs = run(
        repo,
        &[
            "for-each-ref",
            "--format=%(refname) %(objectname)",
            "refs/heads",
            "refs/remotes",
        ],
    )
    .await?;
    let mut hash = Sha256::new();
    hash.update(head);
    hash.update(refs);
    Ok(format!("{:x}", hash.finalize()))
}
pub async fn worktree(repo: &Repo) -> Result<String, ServiceError> {
    let tracked = run(repo, &["diff", "--name-only", "-z", "HEAD"]).await?;
    let untracked = run(repo, &["ls-files", "--others", "--exclude-standard", "-z"]).await?;
    let mut hash = Sha256::new();
    hash.update(&tracked);
    hash.update(&untracked);
    for path in tracked
        .split(|b| *b == 0)
        .chain(untracked.split(|b| *b == 0))
        .filter(|p| !p.is_empty())
    {
        hash.update(path);
        let path = String::from_utf8_lossy(path);
        match tokio::fs::metadata(repo.path.join(path.as_ref()))
            .await
            .and_then(|m| m.modified())
        {
            Ok(time) => {
                let nanos = match time.duration_since(UNIX_EPOCH) {
                    Ok(d) => d.as_nanos() as i128,
                    Err(e) => -(e.duration().as_nanos() as i128),
                };
                hash.update(format!(":{nanos}"));
            }
            Err(_) => hash.update(b":absent"),
        }
    }
    Ok(format!("{:x}", hash.finalize()))
}
