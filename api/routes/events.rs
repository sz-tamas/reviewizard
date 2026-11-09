use crate::{services::http::AppState, utils::errors::ServiceError};
use axum::{
    extract::{Path, State, rejection::PathRejection},
    response::{
        IntoResponse, Sse,
        sse::{Event, KeepAlive},
    },
};
use futures_util::stream;
use std::{convert::Infallible, sync::Arc, time::Duration};
pub async fn get(
    State(state): State<Arc<AppState>>,
    path: Result<Path<i64>, PathRejection>,
) -> Result<impl IntoResponse, ServiceError> {
    let Path(id) = path.map_err(|e| ServiceError::bad(e.body_text()))?;
    // A missing review still gets a stream, without a repository poller.
    let repo = state
        .store
        .run(move |s| s.review_repo_head(id))
        .await
        .ok()
        .map(|(path, _)| path.into());
    let sub = state.live.subscribe(id, repo);
    let events = stream::unfold((sub, true), |(mut sub, first)| async move {
        if first {
            return Some((
                Ok::<_, Infallible>(Event::default().comment("connected")),
                (sub, false),
            ));
        }
        sub.next()
            .await
            .map(|kind| (Ok(Event::default().data(kind)), (sub, false)))
    });
    Ok((
        [(axum::http::header::CONNECTION, "keep-alive")],
        Sse::new(events).keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(25))
                .text("keepalive"),
        ),
    ))
}
