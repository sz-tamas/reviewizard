//! Repository discovery orchestration, usable without an HTTP server.
use crate::{
    services::workspace::{Entry, Workspace},
    utils::errors::ServiceError,
};
use serde::Serialize;
#[derive(Serialize)]
pub struct Repositories {
    repos: Vec<Entry>,
}
pub async fn list(workspace: Workspace) -> Result<Repositories, ServiceError> {
    let repos = tokio::task::spawn_blocking(move || workspace.list())
        .await
        .map_err(ServiceError::internal)??;
    Ok(Repositories { repos })
}
