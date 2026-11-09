//! Branch discovery, parsing and ordering; independent of HTTP.
use crate::{services::workspace::Workspace, utils::errors::ServiceError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[derive(Default, Deserialize)]
#[serde(default)]
pub struct BranchQuery {
    repo: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Branch {
    name: String,
    is_current: bool,
    is_main: bool,
    is_remote: bool,
    last_commit: String,
}
#[derive(Serialize)]
pub struct Branches {
    branches: Vec<Branch>,
}
impl Branch {
    fn timestamp(&self) -> i64 {
        chrono::DateTime::parse_from_rfc3339(&self.last_commit)
            .map(|d| d.timestamp())
            .unwrap_or(i64::MIN)
    }
    fn short(&self) -> &str {
        if self.is_remote {
            self.name
                .split_once('/')
                .map(|(_, s)| s)
                .unwrap_or(&self.name)
        } else {
            &self.name
        }
    }
    fn rank(&self) -> usize {
        ["main", "master", "develop", "development", "dev", "staging"]
            .iter()
            .position(|n| *n == self.short())
            .unwrap_or(6)
    }
    fn group(&self) -> String {
        if self.is_remote {
            let Some((remote, rest)) = self.name.split_once('/') else {
                return self.name.clone();
            };
            format!("{remote}/{}", rest.split('/').next().unwrap_or(""))
        } else {
            self.name.split('/').next().unwrap_or("").to_owned()
        }
    }
}
pub async fn list(workspace: &Workspace, q: BranchQuery) -> Result<Branches, ServiceError> {
    let repo = workspace.repo(&q.repo)?;
    let main = repo.main_branch().await;
    let locals = repo
        .run(&[
            "branch",
            "--format=%(refname:short)\x1f%(committerdate:iso-strict)\x1f%(HEAD)",
        ])
        .await?;
    let remotes = repo
        .run(&[
            "for-each-ref",
            "--format=%(refname:short)\x1f%(symref)\x1f%(committerdate:iso-strict)",
            "refs/remotes",
        ])
        .await?;
    let mut branches = Vec::new();
    for line in String::from_utf8_lossy(&locals).lines() {
        let fields: Vec<_> = line.split('\x1f').collect();
        if let [name, date, current] = fields.as_slice() {
            branches.push(Branch {
                name: name.to_string(),
                is_main: *name == main,
                is_remote: false,
                is_current: current.trim() == "*",
                last_commit: date.to_string(),
            });
        }
    }
    for line in String::from_utf8_lossy(&remotes).lines() {
        let fields: Vec<_> = line.split('\x1f').collect();
        if let [name, symref, date] = fields.as_slice() {
            if !symref.trim().is_empty() {
                continue;
            }
            branches.push(Branch {
                name: name.to_string(),
                is_main: *name == main,
                is_remote: true,
                is_current: false,
                last_commit: date.to_string(),
            });
        }
    }
    sort_branches(&mut branches);
    Ok(Branches { branches })
}

fn sort_branches(branches: &mut [Branch]) {
    let mut newest = HashMap::new();
    for branch in branches.iter() {
        let value = newest
            .entry((branch.is_remote, branch.group()))
            .or_insert(i64::MIN);
        *value = (*value).max(branch.timestamp());
    }
    branches.sort_by(|a, b| {
        let ag = a.group();
        let bg = b.group();
        a.is_remote
            .cmp(&b.is_remote)
            .then_with(|| a.rank().cmp(&b.rank()))
            .then_with(|| {
                if ag != bg {
                    newest[&(b.is_remote, bg.clone())]
                        .cmp(&newest[&(a.is_remote, ag.clone())])
                        .then_with(|| ag.cmp(&bg))
                } else {
                    b.timestamp()
                        .cmp(&a.timestamp())
                        .then_with(|| a.name.cmp(&b.name))
                }
            })
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn branch(name: &str, date: &str, remote: bool) -> Branch {
        Branch {
            name: name.into(),
            last_commit: date.into(),
            is_remote: remote,
            is_current: false,
            is_main: false,
        }
    }
    #[test]
    fn regression_sort_rank_group_and_undated_cases() {
        let mut values = vec![
            branch("abc/old", "2026-01-02T00:00:00Z", false),
            branch("origin/abc/x", "2026-04-01T00:00:00Z", true),
            branch("zzz/only", "2026-02-01T00:00:00Z", false),
            branch("main", "2025-01-01T00:00:00Z", false),
            branch("solo", "2026-03-01T00:00:00Z", false),
            branch("abc/new", "2026-05-01T00:00:00Z", false),
            branch("origin/main", "2026-01-01T00:00:00Z", true),
            branch("origin/staging", "2026-01-01T00:00:00Z", true),
        ];
        sort_branches(&mut values);
        assert_eq!(
            values.iter().map(|b| b.name.as_str()).collect::<Vec<_>>(),
            [
                "main",
                "abc/new",
                "abc/old",
                "solo",
                "zzz/only",
                "origin/main",
                "origin/staging",
                "origin/abc/x"
            ]
        );
        for (name, remote, rank, group) in [
            ("main", false, 0, "main"),
            ("origin/main", true, 0, "origin/main"),
            ("upstream/main", true, 0, "upstream/main"),
            ("origin/development", true, 3, "origin/development"),
            ("origin/abc/main", true, 6, "origin/abc"),
            ("origin/main", false, 6, "origin"),
            ("feature/x", false, 6, "feature"),
            ("solo", false, 6, "solo"),
            ("weird", true, 6, "weird"),
        ] {
            let b = branch(name, "", remote);
            assert_eq!(b.rank(), rank, "{name}");
            assert_eq!(b.group(), group, "{name}");
        }
        let mut values = vec![
            branch("undated", "", false),
            branch("dated", "2020-01-01T00:00:00Z", false),
        ];
        sort_branches(&mut values);
        assert_eq!(values[0].name, "dated");
    }
}
