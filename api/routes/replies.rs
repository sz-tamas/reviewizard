use crate::services::http::body::CompatJson;
use crate::{
    services::{http::AppState, mutations::*, store::Reply},
    utils::errors::ServiceError,
};
use axum::{
    Json,
    extract::{Path, State, rejection::PathRejection},
    http::StatusCode,
};
use std::sync::Arc;
pub async fn add(
    State(state): State<Arc<AppState>>,
    path: Result<Path<i64>, PathRejection>,
    input: Result<CompatJson<ReplyRequest>, ServiceError>,
) -> Result<Json<Reply>, ServiceError> {
    Ok(Json(
        state
            .mutations()
            .add_reply(super::input::id(path)?, super::input::body(input)?)
            .await?,
    ))
}
pub async fn update(
    State(state): State<Arc<AppState>>,
    path: Result<Path<i64>, PathRejection>,
    input: Result<CompatJson<ReplyRequest>, ServiceError>,
) -> Result<Json<Reply>, ServiceError> {
    Ok(Json(
        state
            .mutations()
            .update_reply(super::input::id(path)?, super::input::body(input)?)
            .await?,
    ))
}
pub async fn delete(
    State(state): State<Arc<AppState>>,
    path: Result<Path<i64>, PathRejection>,
) -> Result<StatusCode, ServiceError> {
    state
        .mutations()
        .delete_reply(super::input::id(path)?)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
