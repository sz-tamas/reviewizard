use crate::{
    services::{
        http::AppState,
        repos::{self, Repositories},
    },
    utils::errors::ServiceError,
};
use axum::{Json, extract::State};
use std::sync::Arc;
pub async fn list(State(state): State<Arc<AppState>>) -> Result<Json<Repositories>, ServiceError> {
    Ok(Json(repos::list(state.workspace.clone()).await?))
}
