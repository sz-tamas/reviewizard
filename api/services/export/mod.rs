//! Canonical review Markdown and optional external-agent reply instructions.
use crate::{
    services::{
        review::{self, DiffCache},
        store::{Comment, Review, Store},
    },
    utils::{
        errors::ServiceError,
        markdown::{fence, filename_component, inline_field, short_sha},
    },
};
use serde::Serialize;
use std::{collections::BTreeMap, fmt::Write};
mod languages;
fn line_label(start: i64, end: i64) -> String {
    if end > start {
        format!("L{start}–{end}")
    } else {
        format!("L{start}")
    }
}
fn anchor_label(c: &Comment) -> String {
    if c.start_line == 0 {
        return "file".into();
    }
    match c.anchor_status.as_str() {
        "moved" => {
            let origin = line_label(c.start_line, c.end_line);
            let origin = if c.current_file_path.is_empty() {
                origin
            } else {
                format!("{}:{origin}", c.file_path)
            };
            format!(
                "{} (moved from {origin})",
                line_label(c.current_start_line, c.current_end_line)
            )
        }
        "outdated" => format!("{} (outdated)", line_label(c.start_line, c.end_line)),
        _ => line_label(c.start_line, c.end_line),
    }
}
pub fn render(review: &Review, instructions: bool, base_url: &str) -> String {
    let mut out = if review.scope_mode == "codebase" {
        format!(
            "# Review: Entire codebase · {} @ {}\n\n",
            review.head_ref,
            short_sha(&review.head_sha)
        )
    } else {
        format!(
            "# Review: {} → {} @ {}\n\n",
            review.head_ref,
            review.base_ref,
            short_sha(&review.head_sha)
        )
    };
    if review.scope_mode == "codebase" {
        writeln!(
            out,
            "Scope: Entire codebase · Source: {}\n",
            crate::services::review::side_label(review.scope_side, &review.head_ref)
        )
        .unwrap();
    } else if review.scope_configured {
        writeln!(
            out,
            "Source: {} · From: {} (inclusive when a commit is selected)\n",
            crate::services::review::side_label(review.scope_side, &review.head_ref),
            inline_field(&review.scope_from)
        )
        .unwrap();
    }
    if !review.summary.trim().is_empty() {
        write!(out, "**Summary**\n\n{}\n\n", review.summary.trim()).unwrap();
    }
    let unresolved: Vec<_> = review.comments.iter().filter(|c| !c.resolved).collect();
    let mut files = BTreeMap::<&str, Vec<&Comment>>::new();
    for c in &unresolved {
        let path = if c.anchor_status == "moved" && !c.current_file_path.is_empty() {
            &c.current_file_path
        } else {
            &c.file_path
        };
        files.entry(path).or_default().push(c);
    }
    writeln!(
        out,
        "_{} unresolved comment(s) across {} file(s)_",
        unresolved.len(),
        files.len()
    )
    .unwrap();
    let resolved = review.comments.len() - unresolved.len();
    if resolved > 0 {
        writeln!(out, "_{resolved} resolved thread(s) omitted._").unwrap();
    }
    for (path, mut comments) in files {
        write!(out, "\n## {}\n", inline_field(path)).unwrap();
        comments.sort_by_key(|c| c.start_line);
        let language = languages::for_path(path);
        for c in comments {
            write!(
                out,
                "\n### #{} · {} · {} · {}\n",
                c.id,
                anchor_label(c),
                inline_field(&c.comment_type),
                inline_field(&c.author)
            )
            .unwrap();
            if !c.snippet.trim().is_empty() {
                let snippet = c.snippet.trim_end_matches('\n');
                let fence = fence(snippet);
                write!(out, "{fence}{language}\n{snippet}\n{fence}\n").unwrap();
            }
            if !c.body.trim().is_empty() {
                writeln!(out, "{}", c.body.trim()).unwrap();
            }
            for reply in &c.replies {
                write!(
                    out,
                    "\n> **↳ reply #{} · {}**\n",
                    reply.id,
                    inline_field(&reply.author)
                )
                .unwrap();
                if !reply.body.trim().is_empty() {
                    out.push_str(">\n");
                    for line in reply.body.trim().split('\n') {
                        writeln!(out, "> {line}").unwrap();
                    }
                }
            }
        }
    }
    if instructions {
        out.push_str(include_str!("instructions.txt"));
        let url = if base_url.is_empty() {
            "http://127.0.0.1:7777"
        } else {
            base_url
        };
        let id = unresolved
            .first()
            .filter(|c| c.id > 0)
            .map(|c| c.id.to_string())
            .unwrap_or_else(|| "<comment-id>".into());
        write!(out,"```sh\ncurl --fail-with-body -sS -X POST {url}/api/comments/{id}/replies \\\n  -H 'Content-Type: application/json' \\\n  -d '{{\"body\": \"your reply here\"}}'\n```\n").unwrap();
        write!(out, "\nAfter the reply succeeds and the fix is verified:\n\n```sh\ncurl --fail-with-body -sS -X POST {url}/api/comments/{id}/resolved \\\n  -H 'Content-Type: application/json' \\\n  -d '{{\"resolved\":true}}'\n```\n").unwrap();
    }
    out
}
/// Host is copied into an agent's shell command: use a strict character allowlist.
pub fn safe_base_url(host: &str) -> String {
    if host.is_empty()
        || !host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ".-:[]".contains(c))
    {
        return "http://127.0.0.1:7777".into();
    }
    format!("http://{host}")
}
#[derive(Serialize)]
pub struct Export {
    pub markdown: String,
    pub filename: String,
}
pub async fn generate(
    store: &Store,
    id: i64,
    instructions: bool,
    host: &str,
    cache: Option<&DiffCache>,
) -> Result<Export, ServiceError> {
    let review = review::load(store, id, cache).await?;
    let markdown = render(&review, instructions, &safe_base_url(host));
    let filename = format!(
        "code-review-{}-{}.md",
        filename_component(&review.head_ref),
        short_sha(&review.head_sha)
    );
    // Status updates are best-effort once the artifact has been rendered.
    let _ = store.run(move |s| s.set_status(id, "exported")).await;
    Ok(Export { markdown, filename })
}
#[cfg(test)]
mod tests;
