//! Shared application state and middleware; endpoint wiring lives in routes/.
use crate::services::{store::Store, workspace::Workspace};
use axum::{Router, extract::DefaultBodyLimit, middleware};
use std::sync::Arc;
mod errors;
mod logging;
mod origin;
mod query;
pub struct AppState {
    pub workspace: Workspace,
    pub store: Store,
    pub diffs: crate::services::review::DiffCache,
    pub live: crate::services::live::LiveUpdates,
}
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .fallback_service(crate::routes::router().with_state(state))
        .layer(DefaultBodyLimit::max(8 << 20))
        .layer(middleware::from_fn(errors::method_headers))
        .layer(middleware::from_fn(query::first_values))
        .layer(middleware::from_fn(origin::same_origin))
        .layer(middleware::from_fn(logging::errors))
}

impl AppState {
    pub fn mutations(&self) -> crate::services::mutations::Mutations<'_> {
        crate::services::mutations::Mutations {
            store: &self.store,
            workspace: &self.workspace,
            diffs: &self.diffs,
            live: &self.live,
        }
    }
}

pub mod body;
