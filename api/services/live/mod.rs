//! Ref-counted review watchers and nonblocking, coalesced stream notifications.
use crate::services::{
    git::{Repo, fingerprint},
    store::Store,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, Ordering},
    },
    time::Duration,
};
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
    time::Instant,
};
const WATCH_INTERVAL: Duration = Duration::from_millis(1500);
const REFS_INTERVAL: Duration = Duration::from_secs(12);
struct Entry {
    flags: Arc<AtomicU8>,
    wake: mpsc::Sender<()>,
}
struct Watcher {
    users: usize,
    task: JoinHandle<()>,
}
#[derive(Default)]
struct Registry {
    next: u64,
    subscribers: HashMap<i64, HashMap<u64, Entry>>,
    watchers: HashMap<i64, Watcher>,
}
struct Inner {
    registry: Mutex<Registry>,
    shutdown: watch::Sender<bool>,
}
#[derive(Clone)]
pub struct LiveUpdates {
    inner: Arc<Inner>,
}
impl Default for LiveUpdates {
    fn default() -> Self {
        let (shutdown, _) = watch::channel(false);
        Self {
            inner: Arc::new(Inner {
                registry: Mutex::new(Registry::default()),
                shutdown,
            }),
        }
    }
}
pub struct Subscription {
    live: LiveUpdates,
    review: i64,
    id: u64,
    watching: bool,
    flags: Arc<AtomicU8>,
    rx: mpsc::Receiver<()>,
    shutdown: watch::Receiver<bool>,
}
impl Subscription {
    pub async fn next(&mut self) -> Option<&'static str> {
        if *self.shutdown.borrow() {
            return None;
        }
        tokio::select! {
            biased;
            _=self.shutdown.changed()=>None,
            event=self.rx.recv()=>{
                event?;
                let flags=self.flags.swap(0,Ordering::AcqRel);
                Some(if flags&2!=0{"refs"}else if flags&1!=0{"diff"}else{"meta"})
            }
        }
    }
}
impl Drop for Subscription {
    fn drop(&mut self) {
        let mut registry = self
            .live
            .inner
            .registry
            .lock()
            .expect("live registry poisoned");
        if let Some(subs) = registry.subscribers.get_mut(&self.review) {
            subs.remove(&self.id);
            if subs.is_empty() {
                registry.subscribers.remove(&self.review);
            }
        }
        if self.watching
            && let Some(watcher) = registry.watchers.get_mut(&self.review)
        {
            watcher.users -= 1;
            if watcher.users == 0 {
                registry.watchers.remove(&self.review).unwrap().task.abort();
            }
        }
    }
}
impl LiveUpdates {
    pub fn subscribe(&self, review: i64, repo: Option<PathBuf>) -> Subscription {
        let (tx, rx) = mpsc::channel(1);
        let flags = Arc::new(AtomicU8::new(0));
        let mut registry = self.inner.registry.lock().expect("live registry poisoned");
        let id = registry.next;
        registry.next += 1;
        registry.subscribers.entry(review).or_default().insert(
            id,
            Entry {
                flags: flags.clone(),
                wake: tx,
            },
        );
        let watching = repo.is_some() && !*self.inner.shutdown.borrow();
        if watching {
            if let Some(watcher) = registry.watchers.get_mut(&review) {
                watcher.users += 1;
            } else {
                let live = self.clone();
                let repo = Repo {
                    path: repo.unwrap(),
                };
                let task = tokio::spawn(async move {
                    live.poll(review, repo).await;
                });
                registry.watchers.insert(review, Watcher { users: 1, task });
            }
        }
        Subscription {
            live: self.clone(),
            review,
            id,
            watching,
            flags,
            rx,
            shutdown: self.inner.shutdown.subscribe(),
        }
    }
    pub fn publish(&self, review: i64, diff: bool, refs: bool) {
        let registry = self.inner.registry.lock().expect("live registry poisoned");
        if let Some(subs) = registry.subscribers.get(&review) {
            for sub in subs.values() {
                sub.flags
                    .fetch_or(u8::from(diff) | (u8::from(refs) << 1), Ordering::AcqRel);
                let _ = sub.wake.try_send(());
            }
        }
    }
    /// Call only after a committed mutation. Touch failure must not hide a successful write.
    pub async fn notify(&self, store: &Store, review: i64) {
        let _ = store.run(move |s| s.touch(review)).await;
        self.publish(review, false, false);
    }
    pub fn shutdown(&self) {
        self.inner.shutdown.send_replace(true);
        let mut registry = self.inner.registry.lock().expect("live registry poisoned");
        for (_, watcher) in registry.watchers.drain() {
            watcher.task.abort();
        }
    }
    async fn poll(&self, review: i64, repo: Repo) {
        let mut ticker = tokio::time::interval_at(Instant::now() + WATCH_INTERVAL, WATCH_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let (mut seeded, mut tree, mut refs, mut read_at) =
            (false, String::new(), String::new(), Instant::now());
        loop {
            ticker.tick().await;
            let Ok(next_tree) = fingerprint::worktree(&repo).await else {
                continue;
            };
            let moved_tree = next_tree != tree;
            let next_refs = if refs_due(seeded, moved_tree, read_at.elapsed()) {
                let Ok(value) = fingerprint::refs(&repo).await else {
                    continue;
                };
                read_at = Instant::now();
                value
            } else {
                refs.clone()
            };
            let moved_refs = next_refs != refs;
            tree = next_tree;
            refs = next_refs;
            if !seeded {
                seeded = true;
                continue;
            }
            if moved_refs {
                self.publish(review, true, true);
            } else if moved_tree {
                self.publish(review, true, false);
            }
        }
    }
}
fn refs_due(seeded: bool, moved: bool, since: Duration) -> bool {
    !seeded || moved || since >= REFS_INTERVAL
}
#[cfg(test)]
mod tests;
