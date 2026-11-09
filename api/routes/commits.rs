use crate::{
    services::{
        commits::{self, CommitQuery, Commits},
        http::AppState,
    },
    utils::errors::ServiceError,
};
use axum::{
    Json,
    extract::{Query, State, rejection::QueryRejection},
};
use std::sync::Arc;
pub async fn list(
    State(state): State<Arc<AppState>>,
    input: Result<Query<CommitQuery>, QueryRejection>,
) -> Result<Json<Commits>, ServiceError> {
    let Query(query) = input.map_err(|e| ServiceError::bad(e.body_text()))?;
    Ok(Json(commits::list(&state.workspace, query).await?))
}
