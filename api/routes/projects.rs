use crate::{
    services::{
        http::{AppState, body::CompatJson},
        projects::{self, AddProject, FolderQuery, Folders},
        store::Project,
    },
    utils::errors::ServiceError,
};
use axum::{
    Json,
    extract::{
        Path, Query, State,
        rejection::{PathRejection, QueryRejection},
    },
    http::StatusCode,
};
use std::sync::Arc;

pub async fn list(State(state): State<Arc<AppState>>) -> Result<Json<Vec<Project>>, ServiceError> {
    Ok(Json(projects::list(&state.store).await?))
}
pub async fn add(
    State(state): State<Arc<AppState>>,
    input: Result<CompatJson<AddProject>, ServiceError>,
) -> Result<Json<Project>, ServiceError> {
    Ok(Json(
        projects::add(&state.store, &state.workspace, super::input::body(input)?).await?,
    ))
}
pub async fn remove(
    State(state): State<Arc<AppState>>,
    id: Result<Path<i64>, PathRejection>,
) -> Result<StatusCode, ServiceError> {
    projects::remove(&state.store, &state.workspace, super::input::id(id)?).await?;
    Ok(StatusCode::NO_CONTENT)
}
pub async fn folders(
    State(state): State<Arc<AppState>>,
    query: Result<Query<FolderQuery>, QueryRejection>,
) -> Result<Json<Folders>, ServiceError> {
    let Query(query) = query.map_err(|e| ServiceError::bad(e.body_text()))?;
    Ok(Json(
        projects::folders(state.workspace.clone(), query).await?,
    ))
}

pub async fn reviews(
    State(state): State<Arc<AppState>>,
    id: Result<Path<i64>, PathRejection>,
) -> Result<Json<Vec<crate::services::store::Review>>, ServiceError> {
    let id = super::input::id(id)?;
    Ok(Json(state.store.run(move |s| s.project_reviews(id)).await?))
}

pub async fn create_review(
    State(state): State<Arc<AppState>>,
    id: Result<Path<i64>, PathRejection>,
    input: Result<CompatJson<crate::services::review::scopes::CreateScope>, ServiceError>,
) -> Result<Json<crate::services::store::Review>, ServiceError> {
    let id = super::input::id(id)?;
    Ok(Json(
        crate::services::review::scopes::create(
            &state.store,
            &state.workspace,
            id,
            super::input::body(input)?,
        )
        .await?,
    ))
}
