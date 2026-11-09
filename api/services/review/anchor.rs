use super::{
    cache::{DiffCache, DiffReads},
    content::ContentCache,
    snippet,
};
use crate::services::{
    git::{
        Repo,
        diff::{FileDiff, Hunk},
        linemap,
    },
    store::{Comment, Side},
};
use std::collections::HashMap;
fn mark(c: &mut Comment, status: &str, path: &str, start: i64, end: i64) {
    c.anchor_status = status.into();
    c.current_file_path = path.into();
    c.current_start_line = start;
    c.current_end_line = end;
}
fn current(c: &mut Comment) {
    mark(c, "current", "", 0, 0)
}
fn outdated(c: &mut Comment) {
    mark(c, "outdated", "", 0, 0)
}
fn contiguous(c: &mut Comment, hunks: &[Hunk], path: &str) {
    let Some(start) = linemap::map_old_line(hunks, c.start_line) else {
        outdated(c);
        return;
    };
    let limit = c.end_line.min(linemap::old_extent(hunks));
    let mut prev = start;
    for line in c.start_line.saturating_add(1)..=limit {
        let Some(mapped) = linemap::map_old_line(hunks, line) else {
            outdated(c);
            return;
        };
        if prev.checked_add(1) != Some(mapped) {
            outdated(c);
            return;
        }
        prev = mapped;
    }
    let Some(end) = c
        .end_line
        .checked_sub(c.start_line)
        .and_then(|span| start.checked_add(span))
    else {
        outdated(c);
        return;
    };
    if path.is_empty() && start == c.start_line {
        current(c);
    } else {
        mark(c, "moved", path, start, end);
    }
}
fn from_whole(c: &mut Comment, file: Option<&FileDiff>) -> bool {
    let Some(file) = file else {
        current(c);
        return true;
    };
    if file.binary {
        return false;
    }
    match file.status {
        "deleted" => outdated(c),
        "renamed" => contiguous(c, &file.hunks, &file.new_path),
        _ if file.hunks.is_empty() => return false,
        _ => contiguous(c, &file.hunks, ""),
    }
    true
}
async fn by_diff(c: &mut Comment, reads: &mut DiffReads<'_>, whole: bool) -> bool {
    let Some(result) = reads
        .get(&c.commit_sha, if whole { None } else { Some(&c.file_path) })
        .await
    else {
        return false;
    };
    let file = result.entry(&c.file_path);
    if whole {
        return from_whole(c, file);
    }
    let Some(file) = file else {
        current(c);
        return true;
    };
    if file.status == "deleted" {
        let Some(result) = reads.get(&c.commit_sha, None).await else {
            return false;
        };
        let file = result.entry(&c.file_path);
        if file.is_none_or(|f| f.status != "renamed") {
            outdated(c);
            return true;
        }
        return from_whole(c, file);
    }
    if file.binary || file.hunks.is_empty() {
        return false;
    }
    contiguous(c, &file.hunks, "");
    true
}
async fn by_snippet(c: &mut Comment, content: &mut ContentCache<'_>) {
    current(c);
    let snippet = c.snippet.trim_end_matches('\n');
    if snippet.trim().is_empty() {
        return;
    }
    let Some(lines) = content.lines(&c.file_path, c.side).await else {
        outdated(c);
        return;
    };
    let snip: Vec<_> = snippet.split('\n').map(str::to_owned).collect();
    if snippet::position(&lines, c.start_line.saturating_sub(1), &snip) {
        return;
    }
    let starts = snippet::matches(&lines, &snip);
    if starts.len() == 1 {
        mark(
            c,
            "moved",
            "",
            starts[0] as i64 + 1,
            (starts[0] + snip.len()) as i64,
        );
    } else {
        outdated(c);
    }
}
fn trackable(c: &Comment, head: &str) -> bool {
    c.side == Side::Head && c.start_line > 0 && !c.commit_sha.is_empty() && c.commit_sha != head
}
pub(super) async fn annotate(
    repo: &Repo,
    head: &str,
    comments: &mut [Comment],
    content: &mut ContentCache<'_>,
    shared: Option<&DiffCache>,
) {
    let mut counts = HashMap::new();
    for c in comments.iter().filter(|c| trackable(c, head)) {
        *counts.entry(c.commit_sha.clone()).or_insert(0usize) += 1;
    }
    let mut reads = DiffReads::new(repo, head, shared);
    for c in comments {
        if trackable(c, head)
            && by_diff(
                c,
                &mut reads,
                counts.get(&c.commit_sha).is_some_and(|n| *n > 1),
            )
            .await
        {
            continue;
        }
        by_snippet(c, content).await;
    }
}
