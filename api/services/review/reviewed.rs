use super::content::ContentCache;
use crate::services::{
    git::Repo,
    store::{ReviewedFile, Side},
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
pub const ABSENT_CONTENT_HASH: &str = "absent";
pub(super) async fn hash(cache: &mut ContentCache<'_>, path: &str, side: Side) -> String {
    match cache.read(path, side).await {
        Some(bytes) => format!("{:x}", Sha256::digest(bytes.as_slice())),
        None => ABSENT_CONTENT_HASH.into(),
    }
}
pub async fn fingerprint_files(
    repo: &Repo,
    head: &str,
    paths: &[String],
    side: Side,
) -> HashMap<String, String> {
    let mut cache = ContentCache::new(repo, head);
    cache.warm(paths, side).await;
    let mut out = HashMap::new();
    for path in paths.iter().filter(|p| !p.is_empty()) {
        out.insert(path.clone(), hash(&mut cache, path, side).await);
    }
    out
}
pub(super) async fn holds(cache: &mut ContentCache<'_>, mark: &ReviewedFile) -> bool {
    mark.content_hash.is_empty() || hash(cache, &mark.path, mark.side).await == mark.content_hash
}
