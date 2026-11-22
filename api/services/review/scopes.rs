//! Saved review assignments. Persist only after Git validates the scope.
use crate::{
    services::{
        git::{Repo, refs},
        store::{Review, SavedScope, Side, Store},
        workspace::Workspace,
    },
    utils::{errors::ServiceError, validation::valid_ref},
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateScope {
    #[serde(default = "changes")]
    pub mode: String,
    pub head: String,
    #[serde(default)]
    pub base: String,
    #[serde(default = "all")]
    pub from: String,
    pub side: Side,
}
fn changes() -> String {
    "changes".into()
}
fn all() -> String {
    "all".into()
}

async fn resolve(repo: &Repo, reference: &str) -> Result<String, ServiceError> {
    valid_ref(reference)?;
    let spec = format!("{reference}^{{commit}}");
    let args = ["rev-parse", "--verify", "--quiet", &spec];
    let output = crate::utils::process::git_output(&repo.path, &args, None).await?;
    if output.status.code() == Some(1) {
        return Err(ServiceError::bad(format!("Unknown ref: {reference}")));
    }
    Ok(
        String::from_utf8_lossy(&crate::utils::process::checked(output, &args)?)
            .trim()
            .into(),
    )
}

pub async fn validate_scope(
    repo: &Repo,
    head: &str,
    base: &str,
    from: &str,
    side: Side,
    mode: &str,
) -> Result<(String, String, String, String), ServiceError> {
    if !matches!(mode, "changes" | "codebase") {
        return Err(ServiceError::bad("mode must be changes or codebase"));
    }
    valid_ref(head)?;
    // A whole-folder review follows the working tree, even with detached or unborn HEAD.
    let folder = mode == "codebase" && side == Side::Worktree && head == "HEAD";
    let sha = if folder {
        repo.run(&["rev-parse", "--git-dir"]).await?;
        let args = ["rev-parse", "--verify", "--quiet", "HEAD^{commit}"];
        let output = crate::utils::process::git_output(&repo.path, &args, None).await?;
        if output.status.code() == Some(1) {
            String::new()
        } else {
            String::from_utf8_lossy(&crate::utils::process::checked(output, &args)?)
                .trim()
                .into()
        }
    } else {
        resolve(repo, head).await?
    };
    // A whole-codebase assignment saved against HEAD follows the checkout itself.
    if side != Side::Head && !(mode == "codebase" && head == "HEAD") {
        let args = ["symbolic-ref", "--quiet", "--short", "HEAD"];
        let output = crate::utils::process::git_output(&repo.path, &args, None).await?;
        if output.status.code() == Some(1) {
            return Err(ServiceError::bad(
                "Uncommitted reviews require a checked-out local branch",
            ));
        }
        let current = crate::utils::process::checked(output, &args)?;
        if String::from_utf8_lossy(&current).trim() != head {
            return Err(ServiceError::bad(
                "Staged and working-tree reviews require the checked-out branch",
            ));
        }
    }
    if mode == "codebase" {
        if !base.is_empty() || from != "all" {
            return Err(ServiceError::bad(
                "Codebase reviews do not use a base or starting commit",
            ));
        }
        let empty = repo.run(&["hash-object", "-t", "tree", "--stdin"]).await?;
        return Ok((
            String::new(),
            sha,
            String::from_utf8_lossy(&empty).trim().into(),
            "all".into(),
        ));
    }
    let base = if base.is_empty() {
        repo.main_branch().await
    } else {
        base.to_owned()
    };
    valid_ref(&base)?;
    resolve(repo, &base).await?;
    let merge = refs::merge_base(repo, &base, head)
        .await?
        .ok_or_else(|| ServiceError::bad("The head and base have no common history"))?;
    let normalized_from;
    let start = if from == "all" {
        normalized_from = "all".to_owned();
        merge
    } else {
        valid_ref(from)?;
        let from_sha = resolve(repo, from).await?;
        normalized_from = from_sha.clone();
        if refs::merge_base(repo, &from_sha, head).await?.as_deref() != Some(&from_sha) {
            return Err(ServiceError::bad(
                "The starting commit is not an ancestor of the selected branch",
            ));
        }
        refs::parent(repo, &from_sha).await?
    };
    Ok((base, sha, start, normalized_from))
}

pub async fn create(
    store: &Store,
    workspace: &Workspace,
    id: i64,
    input: CreateScope,
) -> Result<Review, ServiceError> {
    let repo = workspace.repo(&format!("@project/{id}"))?;
    let (base, sha, _, from) = validate_scope(
        &repo,
        &input.head,
        &input.base,
        &input.from,
        input.side,
        &input.mode,
    )
    .await?;
    let path = repo.path.to_string_lossy().into_owned();
    store
        .run(move |s| {
            s.save_scoped_review(SavedScope {
                mode: &input.mode,
                project_id: id,
                path: &path,
                head: &input.head,
                base: &base,
                sha: &sha,
                from: &from,
                side: input.side,
            })
        })
        .await
}

pub async fn diff(store: &Store, id: i64) -> Result<crate::services::diff::Diff, ServiceError> {
    let review = store
        .run(move |s| {
            let r = s.get_review(id)?;
            s.validate_registered_repo(&r.repo_path)?;
            Ok(r)
        })
        .await?;
    let repo = Repo {
        path: review.repo_path.into(),
    };
    let (_, _, base, _) = validate_scope(
        &repo,
        &review.head_ref,
        &review.base_ref,
        &review.scope_from,
        review.scope_side,
        &review.scope_mode,
    )
    .await?;
    let mut files = crate::services::git::diff::read(
        &repo,
        &base,
        &review.head_ref,
        review.scope_side != Side::Head,
        review.scope_side != Side::Index,
    )
    .await?;
    crate::services::git::attributes::mark(&repo, &mut files).await;
    Ok(crate::services::diff::Diff {
        base,
        head: review.head_ref,
        files: if files.is_empty() { None } else { Some(files) },
    })
}

/// Explicitly change the saved source only after validating it against the repository.
pub async fn change_side(store: &Store, id: i64, side: Side) -> Result<(), ServiceError> {
    let review = store
        .run(move |s| {
            let review = s.get_review(id)?;
            s.validate_registered_repo(&review.repo_path)?;
            Ok(review)
        })
        .await?;
    validate_scope(
        &Repo {
            path: review.repo_path.into(),
        },
        &review.head_ref,
        &review.base_ref,
        &review.scope_from,
        side,
        &review.scope_mode,
    )
    .await?;
    store.run(move |s| s.set_review_side(id, side)).await
}
