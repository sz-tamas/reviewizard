use super::content::read_side;
use crate::services::{git::Repo, store::Side};
pub(super) fn split_lines(s: &str) -> Vec<String> {
    s.strip_suffix('\n')
        .unwrap_or(s)
        .split('\n')
        .map(str::to_owned)
        .collect()
}
pub async fn capture_snippet(
    repo: Option<&Repo>,
    head: &str,
    path: &str,
    start: i64,
    end: i64,
    side: Side,
) -> String {
    let Some(repo) = repo.filter(|_| start > 0) else {
        return String::new();
    };
    let Ok(bytes) = read_side(repo, head, path, side).await else {
        return String::new();
    };
    let lines = split_lines(&String::from_utf8_lossy(&bytes));
    let Ok(start) = usize::try_from(start - 1) else {
        return String::new();
    };
    if start >= lines.len() {
        return String::new();
    }
    let end = usize::try_from(end)
        .unwrap_or(start + 1)
        .min(lines.len())
        .max(start + 1);
    lines[start..end].join("\n")
}
pub(super) fn position(lines: &[String], start: i64, snippet: &[String]) -> bool {
    usize::try_from(start)
        .ok()
        .and_then(|s| {
            s.checked_add(snippet.len())
                .and_then(|end| lines.get(s..end))
        })
        .is_some_and(|part| part == snippet)
}
pub(super) fn matches(lines: &[String], snippet: &[String]) -> Vec<usize> {
    if snippet.is_empty() {
        return Vec::new();
    }
    lines
        .windows(snippet.len())
        .enumerate()
        .filter(|(_, s)| *s == snippet)
        .map(|(i, _)| i)
        .take(2)
        .collect()
}
