//! Log failed requests without consuming response bodies or buffering SSE streams.
use axum::{extract::Request, middleware::Next, response::Response};
pub async fn errors(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let response = next.run(request).await;
    if response.status().is_client_error() || response.status().is_server_error() {
        eprintln!("http {} {method} {path}", response.status().as_u16());
    }
    response
}
