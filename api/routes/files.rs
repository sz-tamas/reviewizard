use crate::{
    services::{
        files::{self, File, FileQuery, Files},
        http::AppState,
    },
    utils::errors::ServiceError,
};
use axum::{
    Json,
    extract::{Query, State, rejection::QueryRejection},
    http::header,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
fn query(input: Result<Query<FileQuery>, QueryRejection>) -> Result<FileQuery, ServiceError> {
    input
        .map(|Query(q)| q)
        .map_err(|e| ServiceError::bad(e.body_text()))
}
pub async fn list(
    State(state): State<Arc<AppState>>,
    input: Result<Query<FileQuery>, QueryRejection>,
) -> Result<Json<Files>, ServiceError> {
    Ok(Json(files::list(&state.workspace, query(input)?).await?))
}
pub async fn file(
    State(state): State<Arc<AppState>>,
    input: Result<Query<FileQuery>, QueryRejection>,
) -> Result<Json<File>, ServiceError> {
    Ok(Json(files::file(&state.workspace, query(input)?).await?))
}
pub async fn blob(
    State(state): State<Arc<AppState>>,
    input: Result<Query<FileQuery>, QueryRejection>,
) -> Result<Response, ServiceError> {
    let blob = files::blob(&state.workspace, query(input)?).await?;
    Ok((
        [
            (header::CONTENT_TYPE, blob.mime),
            (header::CACHE_CONTROL, "no-cache".into()),
            (
                header::CONTENT_SECURITY_POLICY,
                "default-src 'none'; style-src 'unsafe-inline'; img-src 'self' data:; sandbox"
                    .into(),
            ),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff".into()),
        ],
        blob.content,
    )
        .into_response())
}
