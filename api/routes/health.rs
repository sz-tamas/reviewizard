use axum::Json;
use serde::Serialize;
#[derive(Serialize)]
pub struct Status {
    backend: &'static str,
    stage: &'static str,
    review_api_ready: bool,
}
pub async fn status() -> Json<Status> {
    Json(Status {
        backend: "rust",
        stage: "default-backend",
        review_api_ready: true,
    })
}
