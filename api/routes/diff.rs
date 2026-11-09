use crate::{
    services::{
        diff::{self, Diff, DiffQuery},
        http::AppState,
    },
    utils::errors::ServiceError,
};
use axum::{
    Json,
    extract::{Query, State, rejection::QueryRejection},
};
use std::sync::Arc;
pub async fn get(
    State(state): State<Arc<AppState>>,
    input: Result<Query<DiffQuery>, QueryRejection>,
) -> Result<Json<Diff>, ServiceError> {
    let Query(query) = input.map_err(|e| ServiceError::bad(e.body_text()))?;
    Ok(Json(diff::get(&state.workspace, query).await?))
}
