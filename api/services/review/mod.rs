//! Live derived review state. Annotations never modify stored anchors or fingerprints.
use crate::{
    services::{
        git::{Repo, refs},
        store::{Comment, Review, ReviewedFile, Side, Store},
    },
    utils::errors::ServiceError,
};
use std::{collections::HashMap, path::PathBuf};
mod anchor;
mod cache;
mod content;
mod reviewed;
pub mod scopes;
mod snippet;
pub use cache::DiffCache;
pub use content::{read_side, side_label};
pub use reviewed::{ABSENT_CONTENT_HASH, fingerprint_files};
pub use snippet::capture_snippet;

pub async fn annotate(
    review: &mut Review,
    marks: Option<&[ReviewedFile]>,
    diffs: Option<&DiffCache>,
) {
    let repo = Repo {
        path: PathBuf::from(&review.repo_path),
    };
    if !repo.path.join(".git").exists() {
        review.annotation_error = format!(
            "{} can no longer be read — the repository may have been moved, renamed, or deleted",
            review.repo_path
        );
        return;
    }
    let resolved = if review.scope_mode == "codebase"
        && review.scope_side == Side::Worktree
        && review.head_ref == "HEAD"
    {
        scopes::validate_scope(
            &repo,
            "HEAD",
            &review.base_ref,
            &review.scope_from,
            Side::Worktree,
            "codebase",
        )
        .await
        .map(|(_, sha, _, _)| sha)
    } else {
        refs::resolve(&repo, &review.head_ref).await
    };
    let Ok(head) = resolved else {
        review.annotation_error = format!(
            "branch {} no longer resolves — it may have been deleted, renamed, or is mid-rebase",
            review.head_ref
        );
        return;
    };
    review.annotation_error.clear();
    let mut cache = content::ContentCache::new(&repo, &review.head_ref);
    let mut sides: HashMap<Side, Vec<String>> = HashMap::new();
    for c in &review.comments {
        if c.start_line > 0 && !c.file_path.is_empty() {
            sides.entry(c.side).or_default().push(c.file_path.clone());
        }
    }
    for m in marks.unwrap_or_default() {
        if !m.content_hash.is_empty() && !m.path.is_empty() {
            sides.entry(m.side).or_default().push(m.path.clone());
        }
    }
    for (side, paths) in sides {
        cache.warm(&paths, side).await;
    }
    anchor::annotate(&repo, &head, &mut review.comments, &mut cache, diffs).await;
    if !review.reviewed_files.is_empty()
        && let Some(marks) = marks
    {
        let mut kept = Vec::new();
        for mark in marks {
            if reviewed::holds(&mut cache, mark).await {
                kept.push(mark.path.clone());
            }
        }
        review.reviewed_files = kept;
    }
}
pub async fn annotate_comment(
    repo: Option<&Repo>,
    head_ref: &str,
    head_sha: &str,
    comment: &mut Comment,
) {
    let Some(repo) = repo else { return };
    let head = if head_sha.is_empty() {
        refs::resolve(repo, head_ref).await.unwrap_or_default()
    } else {
        head_sha.into()
    };
    let mut cache = content::ContentCache::new(repo, head_ref);
    anchor::annotate(repo, &head, std::slice::from_mut(comment), &mut cache, None).await;
}
/// Database access runs on blocking workers; repository derivation remains async.
pub async fn load(
    store: &Store,
    id: i64,
    diffs: Option<&DiffCache>,
) -> Result<Review, ServiceError> {
    let (mut review, marks, identity_error) = store
        .run(move |s| {
            let review = s.get_review(id)?;
            let error = match s.validate_registered_repo(&review.repo_path) {
                Ok(()) => None,
                Err(e) if e.kind == crate::utils::errors::ErrorKind::Internal => return Err(e),
                Err(e) => Some(e),
            };
            Ok((review, s.list_reviewed_files_full(id).ok(), error))
        })
        .await?;
    if let Some(error) = identity_error {
        review.annotation_error = error.message;
        return Ok(review);
    }
    annotate(&mut review, marks.as_deref(), diffs).await;
    Ok(review)
}
#[cfg(test)]
mod tests;
