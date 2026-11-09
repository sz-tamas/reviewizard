use crate::services::http::body::CompatJson;
use crate::{
    services::{http::AppState, review, store::Review},
    utils::errors::ServiceError,
};
use axum::{
    Json,
    extract::{Path, State, rejection::PathRejection},
};
use std::sync::Arc;
pub async fn get(
    State(state): State<Arc<AppState>>,
    path: Result<Path<i64>, PathRejection>,
) -> Result<Json<Review>, ServiceError> {
    let Path(id) = path.map_err(|e| ServiceError::bad(e.body_text()))?;
    Ok(Json(
        review::load(&state.store, id, Some(&state.diffs)).await?,
    ))
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    input: Result<CompatJson<crate::services::mutations::CreateReview>, ServiceError>,
) -> Result<Json<Review>, ServiceError> {
    Ok(Json(
        state.mutations().create(super::input::body(input)?).await?,
    ))
}

pub async fn diff(
    State(state): State<Arc<AppState>>,
    id: Result<Path<i64>, PathRejection>,
) -> Result<Json<crate::services::diff::Diff>, ServiceError> {
    Ok(Json(
        review::scopes::diff(&state.store, super::input::id(id)?).await?,
    ))
}
pub async fn status(
    State(state): State<Arc<AppState>>,
    id: Result<Path<i64>, PathRejection>,
    input: Result<CompatJson<crate::services::mutations::Status>, ServiceError>,
) -> Result<axum::http::StatusCode, ServiceError> {
    state
        .mutations()
        .status(super::input::id(id)?, super::input::body(input)?)
        .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
pub async fn reset(
    State(state): State<Arc<AppState>>,
    path: Result<Path<i64>, PathRejection>,
) -> Result<axum::http::StatusCode, ServiceError> {
    state.mutations().reset(super::input::id(path)?).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
pub async fn summary(
    State(state): State<Arc<AppState>>,
    path: Result<Path<i64>, PathRejection>,
    input: Result<CompatJson<crate::services::mutations::Summary>, ServiceError>,
) -> Result<axum::http::StatusCode, ServiceError> {
    state
        .mutations()
        .summary(super::input::id(path)?, super::input::body(input)?)
        .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
pub async fn reviewed(
    State(state): State<Arc<AppState>>,
    path: Result<Path<i64>, PathRejection>,
    input: Result<CompatJson<crate::services::mutations::Reviewed>, ServiceError>,
) -> Result<axum::http::StatusCode, ServiceError> {
    state
        .mutations()
        .reviewed(super::input::id(path)?, super::input::body(input)?)
        .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
