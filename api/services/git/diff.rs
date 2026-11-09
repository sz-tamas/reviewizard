//! Structured Git diffs, with stable wire fields and parser behavior.
use super::Repo;
use crate::utils::errors::ServiceError;
use serde::Serialize;
use tokio::io::AsyncReadExt;

fn is_false(value: &bool) -> bool {
    !value
}
fn is_zero(value: &usize) -> bool {
    *value == 0
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    pub kind: &'static str,
    #[serde(skip_serializing_if = "is_zero")]
    pub old_line: usize,
    #[serde(skip_serializing_if = "is_zero")]
    pub new_line: usize,
    pub content: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct Hunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub old_path: String,
    pub new_path: String,
    pub status: &'static str,
    #[serde(skip_serializing_if = "is_false")]
    pub binary: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub generated: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub language: String,
    #[serde(skip_serializing_if = "is_false")]
    pub too_large: bool,
    pub hunks: Vec<Hunk>,
}
impl FileDiff {
    fn new(old_path: String, new_path: String) -> Self {
        Self {
            old_path,
            new_path,
            status: "modified",
            binary: false,
            generated: false,
            language: String::new(),
            too_large: false,
            hunks: Vec::new(),
        }
    }
    pub fn path(&self) -> &str {
        if self.new_path.is_empty() {
            &self.old_path
        } else {
            &self.new_path
        }
    }
}

pub fn parse(text: &str) -> Result<Vec<FileDiff>, ServiceError> {
    let mut files: Vec<FileDiff> = Vec::new();
    let (mut old, mut new) = (0, 0);
    for raw in text.split_terminator('\n') {
        if raw.len() >= 16 * 1024 * 1024 {
            return Err(ServiceError::internal("parse diff: line exceeds 16 MiB"));
        }
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if let Some(rest) = line.strip_prefix("diff --git ") {
            let (a, b) = rest.split_once(" b/").unwrap_or(("", ""));
            files.push(FileDiff::new(
                a.strip_prefix("a/").unwrap_or(a).into(),
                b.into(),
            ));
            continue;
        }
        let Some(file) = files.last_mut() else {
            continue;
        };
        if line.starts_with("@@") {
            let fields: Vec<_> = line.split(' ').collect();
            let first = |v: &str| {
                v.trim_start_matches(['-', '+'])
                    .split(',')
                    .next()
                    .unwrap_or("")
                    .parse()
                    .unwrap_or(0)
            };
            (old, new) = if fields.len() >= 3 {
                (first(fields[1]), first(fields[2]))
            } else {
                (0, 0)
            };
            file.hunks.push(Hunk {
                header: line.into(),
                lines: Vec::new(),
            });
        } else if let Some(hunk) = file.hunks.last_mut() {
            let (kind, old_line, new_line, content) = match line.as_bytes().first() {
                Some(b'+') => {
                    let n = new;
                    new += 1;
                    ("add", 0, n, &line[1..])
                }
                Some(b'-') => {
                    let n = old;
                    old += 1;
                    ("del", n, 0, &line[1..])
                }
                Some(b'\\') => continue,
                _ => {
                    let (o, n) = (old, new);
                    old += 1;
                    new += 1;
                    ("context", o, n, line.strip_prefix(' ').unwrap_or(line))
                }
            };
            hunk.lines.push(DiffLine {
                kind,
                old_line,
                new_line,
                content: content.into(),
            });
        } else if line.starts_with("new file") {
            file.status = "added";
            file.old_path.clear();
        } else if line.starts_with("deleted file") {
            file.status = "deleted";
            file.new_path.clear();
        } else if let Some(path) = line.strip_prefix("rename from ") {
            file.status = "renamed";
            file.old_path = path.into();
        } else if let Some(path) = line.strip_prefix("rename to ") {
            file.status = "renamed";
            file.new_path = path.into();
        } else if let Some(path) = line.strip_prefix("--- ") {
            file.old_path = strip_path(path);
        } else if let Some(path) = line.strip_prefix("+++ ") {
            file.new_path = strip_path(path);
        } else if line.starts_with("Binary files ") {
            file.binary = true;
        }
    }
    Ok(files)
}
fn strip_path(path: &str) -> String {
    let path = path.trim();
    if path == "/dev/null" {
        return String::new();
    }
    path.strip_prefix("a/")
        .or_else(|| path.strip_prefix("b/"))
        .unwrap_or(path)
        .into()
}

pub async fn read(
    repo: &Repo,
    base: &str,
    head: &str,
    uncommitted: bool,
    unstaged: bool,
) -> Result<Vec<FileDiff>, ServiceError> {
    let mut args = vec![
        "-c",
        "core.quotePath=false",
        "diff",
        "--no-color",
        "--no-ext-diff",
        "--no-textconv",
        "--find-renames",
        "--src-prefix=a/",
        "--dst-prefix=b/",
    ];
    if uncommitted && !unstaged {
        args.push("--cached");
    }
    args.push(base);
    if !uncommitted {
        args.push(head);
    }
    let bytes = repo.run(&args).await?;
    let mut files = parse(&String::from_utf8_lossy(&bytes))?;
    if uncommitted && unstaged {
        let bytes = repo
            .run(&["ls-files", "--others", "--exclude-standard", "-z"])
            .await?;
        for path in bytes.split(|b| *b == 0).filter(|p| !p.is_empty()) {
            if let Some(file) = untracked(repo, &String::from_utf8_lossy(path)).await {
                files.push(file);
            }
        }
    }
    Ok(files)
}
async fn untracked(repo: &Repo, path: &str) -> Option<FileDiff> {
    let full = repo.worktree_path(path).await.ok()?;
    let info = tokio::fs::metadata(&full).await.ok()?;
    let mut fd = FileDiff::new(String::new(), path.into());
    fd.status = "added";
    if info.len() > 1 << 20 {
        let mut head = Vec::new();
        tokio::fs::File::open(full)
            .await
            .ok()?
            .take(8000)
            .read_to_end(&mut head)
            .await
            .ok()?;
        fd.binary = head.contains(&0);
        fd.too_large = !fd.binary;
        return Some(fd);
    }
    let bytes = tokio::fs::read(full).await.ok()?;
    fd.binary = bytes.contains(&0);
    if !fd.binary && !bytes.is_empty() {
        let text = String::from_utf8_lossy(&bytes);
        let lines: Vec<_> = text
            .strip_suffix('\n')
            .unwrap_or(&text)
            .split('\n')
            .enumerate()
            .map(|(i, line)| DiffLine {
                kind: "add",
                old_line: 0,
                new_line: i + 1,
                content: line.into(),
            })
            .collect();
        fd.hunks.push(Hunk {
            header: format!("@@ -0,0 +1,{} @@", lines.len()),
            lines,
        });
    }
    Some(fd)
}

/// Restricted immutable diff for anchor tracking. No attributes or untracked reads.
pub async fn between(
    repo: &Repo,
    from: &str,
    to: &str,
    path: Option<&str>,
) -> Result<Vec<FileDiff>, ServiceError> {
    let mut args = vec![
        "-c",
        "core.quotePath=false",
        "diff",
        "--no-color",
        "--no-ext-diff",
        "--no-textconv",
        "--find-renames",
        "--src-prefix=a/",
        "--dst-prefix=b/",
        from,
        to,
    ];
    if let Some(path) = path {
        args.extend(["--", path]);
    }
    let bytes = repo.run(&args).await?;
    parse(&String::from_utf8_lossy(&bytes))
}
