//! Startup policy and OS integration; main.rs collects configuration and server wiring.
use crate::{
    services::{config::Config, http::AppState, store::Store, workspace::Workspace},
    utils::{errors::ServiceError, paths},
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
pub async fn initialize(config: &Config) -> Result<(Arc<AppState>, PathBuf), ServiceError> {
    let home = paths::home();
    let root = paths::expand_home(&config.root, home.as_deref())?;
    let workspace = Workspace::open(&root).map_err(ServiceError::internal)?;
    let dir = resolve_data_dir(config.data_dir.as_deref(), home.as_deref())?;
    let db_dir = dir.clone();
    let store = tokio::task::spawn_blocking(move || {
        super::transition::prepare(&db_dir)?;
        Store::open_directory(&db_dir)
    })
    .await
    .map_err(ServiceError::internal)??;
    for project in store.run(|s| s.projects()).await? {
        workspace.register(project)?;
    }
    apply_retention(&store, config.retention_days).await?;
    Ok((
        Arc::new(AppState {
            workspace,
            store,
            diffs: Default::default(),
            live: Default::default(),
        }),
        dir.join("reviewizard.db"),
    ))
}
pub fn resolve_data_dir(dir: Option<&Path>, home: Option<&Path>) -> Result<PathBuf, ServiceError> {
    let dir = match dir.filter(|d| !d.as_os_str().is_empty()) {
        Some(dir) => paths::expand_home(dir, home)?,
        None => {
            let home =
                home.ok_or_else(|| ServiceError::internal("cannot resolve home directory"))?;
            let canonical = home.join(".reviewizard");
            if home.join(".reviewizard-rust/reviewizard.db").exists() {
                return Err(ServiceError::bad(
                    "An isolated Rust database exists in ~/.reviewizard-rust. Select --data-dir ~/.reviewizard or --data-dir ~/.reviewizard-rust explicitly; databases are never merged automatically.",
                ));
            }
            canonical
        }
    };
    paths::absolute_clean(&dir).map_err(ServiceError::internal)
}
pub async fn apply_retention(store: &Store, days: i64) -> Result<(), ServiceError> {
    if days <= 0 {
        eprintln!("retention-days <= 0: draft pruning disabled");
        return Ok(());
    }
    let seconds = (days as u64)
        .checked_mul(86400)
        .ok_or_else(|| ServiceError::bad("retention-days is too large"))?;
    match store
        .run(move |s| s.prune_drafts(Duration::from_secs(seconds)))
        .await
    {
        Ok(n) if n > 0 => eprintln!("pruned {n} stale draft review(s)"),
        Err(e) => eprintln!("prune drafts: {e}"),
        _ => {}
    }
    Ok(())
}
pub async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install SIGTERM handler");
        tokio::select! {_=tokio::signal::ctrl_c()=>{},_=terminate.recv()=>{}}
    }
    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_data_dir_and_explicit_home_expansion() {
        let home = std::env::current_dir().unwrap().join("fake-home");
        assert_eq!(
            resolve_data_dir(None, Some(&home)).unwrap(),
            home.join(".reviewizard")
        );
        assert_eq!(
            resolve_data_dir(Some(Path::new("~/reviews")), Some(&home)).unwrap(),
            home.join("reviews")
        );
        assert!(resolve_data_dir(None, None).is_err());
    }
}
