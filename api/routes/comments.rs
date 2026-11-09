use crate::services::http::body::CompatJson;
use crate::{
    services::{http::AppState, mutations::*, store::Comment},
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
    input: Result<CompatJson<AddComment>, ServiceError>,
) -> Result<Json<Comment>, ServiceError> {
    Ok(Json(
        state
            .mutations()
            .add_comment(super::input::id(path)?, super::input::body(input)?)
            .await?,
    ))
}
pub async fn update(
    State(state): State<Arc<AppState>>,
    path: Result<Path<i64>, PathRejection>,
    input: Result<CompatJson<UpdateComment>, ServiceError>,
) -> Result<Json<Comment>, ServiceError> {
    Ok(Json(
        state
            .mutations()
            .update_comment(super::input::id(path)?, super::input::body(input)?)
            .await?,
    ))
}
pub async fn resolved(
    State(state): State<Arc<AppState>>,
    path: Result<Path<i64>, PathRejection>,
    input: Result<CompatJson<Resolved>, ServiceError>,
) -> Result<StatusCode, ServiceError> {
    state
        .mutations()
        .resolved(super::input::id(path)?, super::input::body(input)?)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
pub async fn delete(
    State(state): State<Arc<AppState>>,
    path: Result<Path<i64>, PathRejection>,
) -> Result<StatusCode, ServiceError> {
    state
        .mutations()
        .delete_comment(super::input::id(path)?)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Default, serde::Deserialize)]
#[serde(default)]
pub struct Filter {
    author: String,
}
#[derive(serde::Serialize)]
pub struct Comments {
    comments: Vec<Comment>,
}
pub async fn list(
    State(state): State<Arc<AppState>>,
    path: Result<Path<i64>, PathRejection>,
    query: Result<axum::extract::Query<Filter>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<Comments>, ServiceError> {
    let id = super::input::id(path)?;
    let axum::extract::Query(filter) = query.map_err(|e| ServiceError::bad(e.body_text()))?;
    let review = crate::services::review::load(&state.store, id, Some(&state.diffs)).await?;
    Ok(Json(Comments {
        comments: review
            .comments
            .into_iter()
            .filter(|c| filter.author.is_empty() || c.author == filter.author)
            .collect(),
    }))
}
