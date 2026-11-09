use super::Repo;
use crate::utils::{errors::ServiceError, process};
pub const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";
// Distinguish unrelated history (exit 1) from invalid refs or Git failures.
pub async fn merge_base(
    repo: &Repo,
    base: &str,
    head: &str,
) -> Result<Option<String>, ServiceError> {
    let args = ["merge-base", base, head];
    let output = process::git_output(&repo.path, &args, None).await?;
    if output.status.code() == Some(1) {
        return Ok(None);
    }
    Ok(Some(
        String::from_utf8_lossy(&process::checked(output, &args)?)
            .trim()
            .into(),
    ))
}
pub async fn parent(repo: &Repo, reference: &str) -> Result<String, ServiceError> {
    let bytes = repo
        .run(&[
            "rev-list",
            "--parents",
            "-n",
            "1",
            &format!("{reference}^{{commit}}"),
        ])
        .await?;
    let text = String::from_utf8_lossy(&bytes);
    let fields: Vec<_> = text.split_whitespace().collect();
    match fields.as_slice() {
        [] => Err(ServiceError::internal(format!(
            "no commit for {reference:?}"
        ))),
        [_] => Ok(EMPTY_TREE.into()),
        [_, parent, ..] => Ok((*parent).into()),
    }
}

pub async fn resolve(repo: &Repo, reference: &str) -> Result<String, ServiceError> {
    crate::utils::validation::valid_ref(reference)?;
    let bytes = repo
        .run(&["rev-parse", "--verify", &format!("{reference}^{{commit}}")])
        .await?;
    Ok(String::from_utf8_lossy(&bytes).trim().into())
}
