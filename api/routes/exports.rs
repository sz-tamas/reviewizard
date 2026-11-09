use crate::{
    services::{
        export::{self, Export},
        http::AppState,
    },
    utils::errors::ServiceError,
};
use axum::{
    Json,
    extract::{
        Path, Query, State,
        rejection::{PathRejection, QueryRejection},
    },
    http::{HeaderMap, header},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use std::sync::Arc;
#[derive(Default, Deserialize)]
#[serde(default)]
pub struct Options {
    instructions: String,
}
async fn generate(
    state: Arc<AppState>,
    path: Result<Path<i64>, PathRejection>,
    query: Result<Query<Options>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Export, ServiceError> {
    let Path(id) = path.map_err(|e| ServiceError::bad(e.body_text()))?;
    let Query(options) = query.map_err(|e| ServiceError::bad(e.body_text()))?;
    let host = headers
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    export::generate(
        &state.store,
        id,
        options.instructions == "true",
        host,
        Some(&state.diffs),
    )
    .await
}
pub async fn json(
    State(state): State<Arc<AppState>>,
    path: Result<Path<i64>, PathRejection>,
    query: Result<Query<Options>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Json<Export>, ServiceError> {
    Ok(Json(generate(state, path, query, headers).await?))
}
pub async fn markdown(
    State(state): State<Arc<AppState>>,
    path: Result<Path<i64>, PathRejection>,
    query: Result<Query<Options>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ServiceError> {
    let export = generate(state, path, query, headers).await?;
    let disposition =
        axum::http::HeaderValue::from_str(&format!("inline; filename=\"{}\"", export.filename))
            .map_err(ServiceError::internal)?;
    let mut response = export.markdown.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("text/markdown; charset=utf-8"),
    );
    response
        .headers_mut()
        .insert(header::CONTENT_DISPOSITION, disposition);
    Ok(response)
}
