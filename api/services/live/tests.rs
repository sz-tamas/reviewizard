use super::*;
#[test]
fn ref_poll_cadence() {
    assert!(refs_due(false, false, Duration::ZERO));
    assert!(!refs_due(true, false, Duration::ZERO));
    assert!(!refs_due(true, false, Duration::from_secs(11)));
    assert!(refs_due(true, false, REFS_INTERVAL));
    assert!(refs_due(true, true, Duration::ZERO));
}
#[tokio::test]
async fn coalescing_priorities_isolation_and_nonblocking_slow_clients() {
    let live = LiveUpdates::default();
    let mut first = live.subscribe(1, None);
    let mut second = live.subscribe(1, None);
    let mut other = live.subscribe(2, None);
    for _ in 0..1000 {
        live.publish(1, false, false);
        live.publish(1, true, false);
        live.publish(1, true, true);
    }
    assert_eq!(first.next().await, Some("refs"));
    assert_eq!(second.next().await, Some("refs"));
    assert!(
        tokio::time::timeout(Duration::from_millis(20), other.next())
            .await
            .is_err()
    );
    live.publish(1, true, false);
    live.publish(1, false, false);
    assert_eq!(first.next().await, Some("diff"));
    live.publish(1, false, false);
    assert_eq!(first.next().await, Some("meta"));
    drop(first);
    drop(second);
    assert!(
        !live
            .inner
            .registry
            .lock()
            .unwrap()
            .subscribers
            .contains_key(&1)
    );
    live.publish(1, true, true);
    drop(other);
    assert!(live.inner.registry.lock().unwrap().subscribers.is_empty());
}
#[tokio::test(start_paused = true)]
async fn watcher_reference_counts_and_shutdown_cleanup() {
    let live = LiveUpdates::default();
    let first = live.subscribe(1, Some(PathBuf::from("unused")));
    let mut second = live.subscribe(1, Some(PathBuf::from("unused")));
    assert_eq!(live.inner.registry.lock().unwrap().watchers[&1].users, 2);
    drop(first);
    assert_eq!(live.inner.registry.lock().unwrap().watchers[&1].users, 1);
    live.shutdown();
    assert_eq!(second.next().await, None);
    drop(second);
    let registry = live.inner.registry.lock().unwrap();
    assert!(registry.watchers.is_empty());
    assert!(registry.subscribers.is_empty());
}
#[tokio::test(start_paused = true)]
async fn last_tab_aborts_watcher() {
    let live = LiveUpdates::default();
    let sub = live.subscribe(1, Some(PathBuf::from("unused")));
    drop(sub);
    assert!(live.inner.registry.lock().unwrap().watchers.is_empty());
}
