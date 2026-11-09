//! Validated review mutations. Notify only after persistence succeeds.
use crate::{
    services::{
        diff,
        git::{Repo, refs},
        live::LiveUpdates,
        review::{self, DiffCache},
        store::{Comment, CommentEdit, FileReviewMark, Reply, Review, Side, Store},
        workspace::Workspace,
    },
    utils::{
        errors::ServiceError,
        validation::{valid_ref, validate_path},
    },
};
use serde::Deserialize;
use std::path::PathBuf;
pub struct Mutations<'a> {
    pub store: &'a Store,
    pub workspace: &'a Workspace,
    pub diffs: &'a DiffCache,
    pub live: &'a LiveUpdates,
}
#[derive(Default, Deserialize)]
#[serde(default)]
pub struct CreateReview {
    pub repo: String,
    pub head: String,
    pub base: String,
}
#[derive(Deserialize)]
pub struct Status {
    pub status: String,
}
#[derive(Default, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct AddComment {
    pub file_path: String,
    pub start_line: i64,
    pub end_line: i64,
    #[serde(rename = "type")]
    pub comment_type: String,
    pub body: String,
    pub author: String,
    pub side: String,
}
#[derive(Default, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateComment {
    pub body: Option<String>,
    #[serde(rename = "type")]
    pub comment_type: Option<String>,
    pub start_line: Option<i64>,
    pub end_line: Option<i64>,
}
#[derive(Default, Deserialize)]
#[serde(default)]
pub struct ReplyRequest {
    pub body: String,
    pub author: String,
}
#[derive(Default, Deserialize)]
#[serde(default)]
pub struct Summary {
    pub summary: String,
}
#[derive(Default, Deserialize)]
#[serde(default)]
pub struct Resolved {
    pub resolved: bool,
}
#[derive(Default, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Reviewed {
    pub file_paths: Vec<String>,
    pub reviewed: bool,
    pub side: String,
}
fn body(value: &str) -> Result<(), ServiceError> {
    if value.trim().is_empty() {
        Err(ServiceError::bad("body is required"))
    } else {
        Ok(())
    }
}
fn kind(value: &str) -> Result<(), ServiceError> {
    if ["general", "bug", "suggestion", "question", "nit"].contains(&value) {
        Ok(())
    } else {
        Err(ServiceError::bad("invalid comment type"))
    }
}
fn start(value: i64) -> Result<(), ServiceError> {
    if value < 0 {
        Err(ServiceError::bad("startLine must be >= 0"))
    } else {
        Ok(())
    }
}
fn side(value: &str) -> Result<Side, ServiceError> {
    Side::parse(value)
        .ok_or_else(|| ServiceError::bad("invalid side: want \"head\", \"worktree\" or \"index\""))
}
impl Mutations<'_> {
    pub async fn status(&self, id: i64, q: Status) -> Result<(), ServiceError> {
        if !matches!(q.status.as_str(), "draft" | "completed") {
            return Err(ServiceError::bad("status must be draft or completed"));
        }
        self.store
            .run(move |s| s.set_review_status(id, &q.status))
            .await?;
        self.live.notify(self.store, id).await;
        Ok(())
    }
    async fn scope_side(&self, id: i64, side: Side) -> Result<(), ServiceError> {
        let r = self
            .store
            .run(move |s| {
                let r = s.get_review(id)?;
                if r.scope_configured && r.scope_side != side {
                    return Err(ServiceError::bad(
                        "The source side must match the saved review scope",
                    ));
                }
                s.validate_registered_repo(&r.repo_path)?;
                Ok(r)
            })
            .await?;
        if r.scope_configured {
            let repo = Repo {
                path: r.repo_path.into(),
            };
            review::scopes::validate_scope(
                &repo,
                &r.head_ref,
                &r.base_ref,
                &r.scope_from,
                side,
                &r.scope_mode,
            )
            .await?;
        }
        Ok(())
    }
    async fn repo(&self, id: i64) -> Result<(Repo, String), ServiceError> {
        let (path, head) = self.store.run(move |s| s.review_repo_head(id)).await?;
        Ok((
            Repo {
                path: PathBuf::from(path),
            },
            head,
        ))
    }
    pub async fn create(&self, q: CreateReview) -> Result<Review, ServiceError> {
        let repo = self.workspace.repo(&q.repo)?;
        valid_ref(&q.head)?;
        if !q.base.is_empty() {
            valid_ref(&q.base)?;
        }
        let sha=refs::resolve(&repo,&q.head).await.map_err(|_|ServiceError::bad(format!("could not resolve branch {:?} — it may have been deleted, renamed, or is mid-rebase; reload to refresh the branch list",q.head)))?;
        let (base, _) = diff::comparison_base(&repo, &q.base, &q.head).await?;
        let path = repo.path.to_string_lossy().into_owned();
        let r = self
            .store
            .run(move |s| s.create_or_get_review(&path, &base, &q.head, &sha))
            .await?;
        review::load(self.store, r.id, Some(self.diffs)).await
    }
    pub async fn reset(&self, id: i64) -> Result<(), ServiceError> {
        self.store.run(move |s| s.reset_review(id)).await?;
        self.live.notify(self.store, id).await;
        Ok(())
    }
    pub async fn summary(&self, id: i64, q: Summary) -> Result<(), ServiceError> {
        self.store
            .run(move |s| s.set_review_summary(id, q.summary.trim()))
            .await?;
        self.live.notify(self.store, id).await;
        Ok(())
    }
    pub async fn reviewed(&self, id: i64, q: Reviewed) -> Result<(), ServiceError> {
        let side = side(&q.side)?;
        if q.reviewed {
            self.scope_side(id, side).await?;
        }
        let hashes = if q.reviewed {
            let (repo, head) = self.repo(id).await?;
            review::fingerprint_files(&repo, &head, &q.file_paths, side).await
        } else {
            Default::default()
        };
        let marks: Vec<_> = q
            .file_paths
            .into_iter()
            .filter(|p| !p.is_empty())
            .map(|p| FileReviewMark {
                content_hash: hashes.get(&p).cloned().unwrap_or_default(),
                path: p,
            })
            .collect();
        if marks.is_empty() {
            return Err(ServiceError::bad("filePaths is required"));
        }
        self.store
            .run(move |s| s.set_files_reviewed(id, &marks, q.reviewed, side))
            .await?;
        self.live.notify(self.store, id).await;
        Ok(())
    }
    pub async fn add_comment(&self, id: i64, mut q: AddComment) -> Result<Comment, ServiceError> {
        start(q.start_line)?;
        validate_path(&q.file_path)?;
        body(&q.body)?;
        let side = side(&q.side)?;
        self.scope_side(id, side).await?;
        q.end_line = q.end_line.max(q.start_line);
        if q.comment_type.is_empty() {
            q.comment_type = "general".into();
        }
        kind(&q.comment_type)?;
        if q.author.is_empty() {
            q.author = "agent".into();
        }
        let (repo, head) = self.repo(id).await?;
        let sha = refs::resolve(&repo, &head).await.unwrap_or_default();
        let snippet = review::capture_snippet(
            Some(&repo),
            &head,
            &q.file_path,
            q.start_line,
            q.end_line,
            side,
        )
        .await;
        let c = Comment {
            review_id: id,
            file_path: q.file_path,
            start_line: q.start_line,
            end_line: q.end_line,
            snippet,
            comment_type: q.comment_type,
            body: q.body,
            author: q.author,
            commit_sha: sha.clone(),
            side,
            ..Default::default()
        };
        let mut c = self.store.run(move |s| s.add_comment(c)).await?;
        review::annotate_comment(Some(&repo), &head, &sha, &mut c).await;
        self.live.notify(self.store, id).await;
        Ok(c)
    }
    pub async fn update_comment(&self, id: i64, q: UpdateComment) -> Result<Comment, ServiceError> {
        let existing = self.store.run(move |s| s.get_comment(id)).await?;
        let body_value = if let Some(value) = q.body {
            body(&value)?;
            value
        } else {
            existing.body.clone()
        };
        let comment_type = if let Some(value) = q.comment_type {
            kind(&value)?;
            value
        } else {
            existing.comment_type.clone()
        };
        let reanchor = q.start_line.is_some() || q.end_line.is_some();
        let start_line = q.start_line.unwrap_or(existing.start_line);
        start(start_line)?;
        let end_line = q.end_line.unwrap_or(existing.end_line).max(start_line);
        let repo_info = self.repo(existing.review_id).await.ok();
        let mut snippet = existing.snippet;
        let mut commit_sha = existing.commit_sha;
        let mut head_sha = String::new();
        if reanchor {
            snippet.clear();
            if start_line > 0
                && let Some((repo, head)) = &repo_info
            {
                snippet = review::capture_snippet(
                    Some(repo),
                    head,
                    &existing.file_path,
                    start_line,
                    end_line,
                    existing.side,
                )
                .await;
                if let Ok(sha) = refs::resolve(repo, head).await {
                    commit_sha = sha.clone();
                    head_sha = sha;
                }
            }
        }
        let edit = CommentEdit {
            body: body_value,
            comment_type,
            start: start_line,
            end: end_line,
            snippet,
            commit_sha,
        };
        let mut c = self.store.run(move |s| s.update_comment(id, edit)).await?;
        if let Some((repo, head)) = &repo_info {
            review::annotate_comment(Some(repo), head, &head_sha, &mut c).await;
        }
        self.live.notify(self.store, c.review_id).await;
        Ok(c)
    }
    pub async fn delete_comment(&self, id: i64) -> Result<(), ServiceError> {
        let review = self.store.run(move |s| s.delete_comment(id)).await?;
        self.live.notify(self.store, review).await;
        Ok(())
    }
    pub async fn resolved(&self, id: i64, q: Resolved) -> Result<(), ServiceError> {
        let review = self
            .store
            .run(move |s| s.set_comment_resolved(id, q.resolved))
            .await?;
        self.live.notify(self.store, review).await;
        Ok(())
    }
    pub async fn add_reply(&self, id: i64, mut q: ReplyRequest) -> Result<Reply, ServiceError> {
        body(&q.body)?;
        if q.author.is_empty() {
            q.author = "agent".into();
        }
        let (reply, review) = self
            .store
            .run(move |s| s.add_reply(id, &q.body, &q.author))
            .await?;
        self.live.notify(self.store, review).await;
        Ok(reply)
    }
    pub async fn update_reply(&self, id: i64, q: ReplyRequest) -> Result<Reply, ServiceError> {
        body(&q.body)?;
        let (reply, review) = self.store.run(move |s| s.update_reply(id, &q.body)).await?;
        self.live.notify(self.store, review).await;
        Ok(reply)
    }
    pub async fn delete_reply(&self, id: i64) -> Result<(), ServiceError> {
        let review = self.store.run(move |s| s.delete_reply(id)).await?;
        self.live.notify(self.store, review).await;
        Ok(())
    }
}
