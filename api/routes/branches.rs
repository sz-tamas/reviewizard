use crate::{
    services::{
        branches::{self, BranchQuery, Branches},
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
    input: Result<Query<BranchQuery>, QueryRejection>,
) -> Result<Json<Branches>, ServiceError> {
    let Query(query) = input.map_err(|e| ServiceError::bad(e.body_text()))?;
    Ok(Json(branches::list(&state.workspace, query).await?))
}
