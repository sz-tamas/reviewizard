use crate::services::git::{
    Repo,
    diff::{self, FileDiff},
};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
};
type Generation = (String, String);
type Key = (String, Option<String>);
#[derive(Debug)]
pub(super) struct DiffResult {
    files: Vec<FileDiff>,
    by_old: HashMap<String, usize>,
}
impl DiffResult {
    fn new(files: Vec<FileDiff>) -> Self {
        let mut by_old = HashMap::new();
        for (i, f) in files.iter().enumerate() {
            by_old.entry(f.old_path.clone()).or_insert(i);
        }
        Self { files, by_old }
    }
    pub fn entry(&self, path: &str) -> Option<&FileDiff> {
        self.by_old.get(path).map(|i| &self.files[*i])
    }
}
#[derive(Default, Debug)]
struct CacheInner {
    gens: HashMap<Generation, HashMap<Key, Arc<DiffResult>>>,
    order: VecDeque<Generation>,
}
#[derive(Default, Debug)]
pub struct DiffCache {
    inner: Mutex<CacheInner>,
}
impl DiffCache {
    pub fn new() -> Self {
        Self::default()
    }
    fn get(&self, generation: &Generation, key: &Key) -> Option<Arc<DiffResult>> {
        self.inner
            .lock()
            .ok()?
            .gens
            .get(generation)?
            .get(key)
            .cloned()
    }
    fn put(&self, generation: Generation, key: Key, result: Arc<DiffResult>) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        if !inner.gens.contains_key(&generation) {
            inner.order.push_back(generation.clone());
        }
        inner
            .gens
            .entry(generation)
            .or_default()
            .insert(key, result);
        while inner.order.len() > 4 {
            if let Some(old) = inner.order.pop_front() {
                inner.gens.remove(&old);
            }
        }
    }
}
pub(super) struct DiffReads<'a> {
    repo: &'a Repo,
    head: &'a str,
    shared: Option<&'a DiffCache>,
    generation: Generation,
    local: HashMap<Key, Option<Arc<DiffResult>>>,
}
impl<'a> DiffReads<'a> {
    pub fn new(repo: &'a Repo, head: &'a str, shared: Option<&'a DiffCache>) -> Self {
        Self {
            repo,
            head,
            shared,
            generation: (repo.path.to_string_lossy().into(), head.into()),
            local: HashMap::new(),
        }
    }
    pub async fn get(&mut self, sha: &str, path: Option<&str>) -> Option<Arc<DiffResult>> {
        let key = (sha.into(), path.map(str::to_owned));
        if let Some(result) = self.local.get(&key) {
            return result.clone();
        }
        if let Some(result) = self.shared.and_then(|c| c.get(&self.generation, &key)) {
            self.local.insert(key, Some(result.clone()));
            return Some(result);
        }
        let result = diff::between(self.repo, sha, self.head, path)
            .await
            .ok()
            .map(DiffResult::new)
            .map(Arc::new);
        if let (Some(cache), Some(result)) = (self.shared, &result) {
            cache.put(self.generation.clone(), key.clone(), result.clone());
        }
        self.local.insert(key, result.clone());
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_generations_and_repository_isolation() {
        let cache = DiffCache::new();
        let value = Arc::new(DiffResult::new(Vec::new()));
        for n in 0..7 {
            cache.put(
                ("repo".into(), n.to_string()),
                ("sha".into(), None),
                value.clone(),
            );
        }
        let inner = cache.inner.lock().unwrap();
        assert_eq!(inner.gens.len(), 4);
        assert!(!inner.gens.contains_key(&("repo".into(), "0".into())));
        drop(inner);
        assert!(
            cache
                .get(
                    &("different-repo".into(), "6".into()),
                    &("sha".into(), None)
                )
                .is_none()
        );
        assert!(Arc::ptr_eq(
            &cache
                .get(&("repo".into(), "6".into()), &("sha".into(), None))
                .unwrap(),
            &value
        ));
    }
    #[tokio::test]
    async fn failed_diffs_are_not_shared() {
        let repo = Repo {
            path: std::env::temp_dir().join("reviewizard-no-such-cache-repo"),
        };
        let cache = DiffCache::new();
        let mut reads = DiffReads::new(&repo, "sha", Some(&cache));
        assert!(reads.get("old", None).await.is_none());
        assert!(cache.inner.lock().unwrap().gens.is_empty());
    }
}

#[cfg(test)]
mod reuse_tests {
    use super::*;
    #[tokio::test]
    async fn repeated_reads_hit_shared_immutable_results_without_git() {
        let repo = Repo {
            path: std::env::temp_dir().join("no-repo-needed-for-cache-hit"),
        };
        let cache = DiffCache::new();
        let value = Arc::new(DiffResult::new(Vec::new()));
        let generation = (repo.path.to_string_lossy().into_owned(), "head-sha".into());
        cache.put(generation, ("old-sha".into(), None), value.clone());
        for _ in 0..2 {
            let mut reads = DiffReads::new(&repo, "head-sha", Some(&cache));
            let hit = reads.get("old-sha", None).await.unwrap();
            assert!(Arc::ptr_eq(&hit, &value));
        }
        let mut other = DiffReads::new(&repo, "new-head", Some(&cache));
        assert!(other.get("old-sha", None).await.is_none());
    }
}
