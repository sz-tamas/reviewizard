//! Collect endpoint modules. main.rs only wires startup, state and this router.
use crate::services::http::AppState;
use axum::{
    Router,
    routing::{get, patch, post},
};
use std::sync::Arc;
mod branches;
mod comments;
mod commits;
mod diff;
pub mod errors;
mod events;
mod exports;
mod files;
mod health;
mod input;
mod projects;
mod replies;
mod repos;
mod reviews;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/healthz", get(health::status))
        .route("/api/repos", get(repos::list))
        .route("/api/projects", get(projects::list).post(projects::add))
        .route(
            "/api/projects/{id}",
            axum::routing::delete(projects::remove),
        )
        .route("/api/project-folders", get(projects::folders))
        .route(
            "/api/projects/{id}/reviews",
            get(projects::reviews).post(projects::create_review),
        )
        .route("/api/branches", get(branches::list))
        .route("/api/commits", get(commits::list))
        .route("/api/files", get(files::list))
        .route("/api/file", get(files::file))
        .route("/api/blob", get(files::blob))
        .route("/api/diff", get(diff::get))
        .route("/api/reviews", post(reviews::create))
        .route(
            "/api/reviews/{id}",
            get(reviews::get).delete(reviews::delete),
        )
        .route("/api/reviews/{id}/diff", get(reviews::diff))
        .route("/api/reviews/{id}/status", post(reviews::status))
        .route("/api/reviews/{id}/source", post(reviews::source))
        .route("/api/reviews/{id}/events", get(events::get))
        .route("/api/reviews/{id}/export", post(exports::json))
        .route("/api/reviews/{id}/export.md", post(exports::markdown))
        .route("/api/reviews/{id}/reset", post(reviews::reset))
        .route("/api/reviews/{id}/reviewed", post(reviews::reviewed))
        .route("/api/reviews/{id}/summary", post(reviews::summary))
        .route(
            "/api/reviews/{id}/comments",
            get(comments::list).post(comments::add),
        )
        .route(
            "/api/comments/{id}",
            patch(comments::update).delete(comments::delete),
        )
        .route("/api/comments/{id}/resolved", post(comments::resolved))
        .route("/api/comments/{id}/replies", post(replies::add))
        .route(
            "/api/replies/{id}",
            patch(replies::update).delete(replies::delete),
        )
        .method_not_allowed_fallback(errors::not_found)
        .fallback(errors::not_found)
}
