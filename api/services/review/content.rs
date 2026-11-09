use crate::{
    services::{
        git::{Repo, batch},
        store::Side,
    },
    utils::errors::ServiceError,
};
use std::{collections::HashMap, sync::Arc};
pub async fn read_side(
    repo: &Repo,
    head: &str,
    path: &str,
    side: Side,
) -> Result<Vec<u8>, ServiceError> {
    match side {
        Side::Head => repo.object(&format!("{head}:{path}")).await,
        Side::Index => repo.object(&format!(":{path}")).await,
        Side::Worktree => repo.worktree_file(path).await,
    }
}
pub fn side_label(side: Side, head: &str) -> &str {
    match side {
        Side::Head => head,
        Side::Index => "the git index",
        Side::Worktree => "the working tree",
    }
}
pub(super) struct ContentCache<'a> {
    repo: &'a Repo,
    head: &'a str,
    entries: HashMap<(Side, String), Option<Arc<Vec<u8>>>>,
    lines: HashMap<(Side, String), Option<Arc<Vec<String>>>>,
}
impl<'a> ContentCache<'a> {
    pub fn new(repo: &'a Repo, head: &'a str) -> Self {
        Self {
            repo,
            head,
            entries: HashMap::new(),
            lines: HashMap::new(),
        }
    }
    fn spec(&self, path: &str, side: Side) -> String {
        if side == Side::Index {
            format!(":{path}")
        } else {
            format!("{}:{path}", self.head)
        }
    }
    pub async fn warm(&mut self, paths: &[String], side: Side) {
        if side == Side::Worktree || paths.is_empty() {
            return;
        }
        let specs: Vec<_> = paths.iter().map(|p| self.spec(p, side)).collect();
        let Ok(mut objects) = batch::objects(self.repo, &specs).await else {
            return;
        };
        for path in paths {
            let spec = self.spec(path, side);
            self.entries
                .entry((side, path.clone()))
                .or_insert_with(|| objects.remove(&spec).map(Arc::new));
        }
    }
    pub async fn read(&mut self, path: &str, side: Side) -> Option<Arc<Vec<u8>>> {
        let key = (side, path.into());
        if let Some(value) = self.entries.get(&key) {
            return value.clone();
        }
        let value = read_side(self.repo, self.head, path, side)
            .await
            .ok()
            .map(Arc::new);
        self.entries.insert(key, value.clone());
        value
    }
    pub async fn lines(&mut self, path: &str, side: Side) -> Option<Arc<Vec<String>>> {
        let key = (side, path.into());
        if let Some(lines) = self.lines.get(&key) {
            return lines.clone();
        }
        let lines = self
            .read(path, side)
            .await
            .map(|b| Arc::new(super::snippet::split_lines(&String::from_utf8_lossy(&b))));
        self.lines.insert(key, lines.clone());
        lines
    }
}
