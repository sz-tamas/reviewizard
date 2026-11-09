use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    #[default]
    Head,
    Worktree,
    Index,
}
impl Side {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "" | "head" => Some(Self::Head),
            "worktree" => Some(Self::Worktree),
            "index" => Some(Self::Index),
            _ => None,
        }
    }
    pub fn flags(self) -> (bool, bool) {
        (self == Self::Worktree, self == Self::Index)
    }
    pub fn from_flags(worktree: bool, indexed: bool) -> Self {
        if indexed {
            Self::Index
        } else if worktree {
            Self::Worktree
        } else {
            Self::Head
        }
    }
}
#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    pub id: i64,
    pub project_id: Option<i64>,
    pub scope_mode: String,
    pub scope_from: String,
    pub scope_side: Side,
    pub scope_configured: bool,
    pub repo_path: String,
    pub base_ref: String,
    pub head_ref: String,
    pub head_sha: String,
    pub status: String,
    pub summary: String,
    pub created_at: String,
    pub updated_at: String,
    pub comments: Vec<Comment>,
    pub reviewed_files: Vec<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub annotation_error: String,
}
#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub id: i64,
    pub review_id: i64,
    pub file_path: String,
    pub start_line: i64,
    pub end_line: i64,
    pub snippet: String,
    #[serde(rename = "type")]
    pub comment_type: String,
    pub body: String,
    pub author: String,
    pub resolved: bool,
    pub commit_sha: String,
    pub side: Side,
    pub created_at: String,
    pub updated_at: String,
    pub replies: Vec<Reply>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub anchor_status: String,
    #[serde(skip_serializing_if = "is_zero")]
    pub current_start_line: i64,
    #[serde(skip_serializing_if = "is_zero")]
    pub current_end_line: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub current_file_path: String,
}
fn is_zero(value: &i64) -> bool {
    *value == 0
}
#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reply {
    pub id: i64,
    pub comment_id: i64,
    pub body: String,
    pub author: String,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedFile {
    pub path: String,
    pub content_hash: String,
    pub side: Side,
}
#[derive(Debug, Clone)]
pub struct FileReviewMark {
    pub path: String,
    pub content_hash: String,
}
#[derive(Debug)]
pub struct CommentEdit {
    pub body: String,
    pub comment_type: String,
    pub start: i64,
    pub end: i64,
    pub snippet: String,
    pub commit_sha: String,
}
