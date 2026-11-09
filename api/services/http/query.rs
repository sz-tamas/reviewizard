//! Query parameters use the first occurrence of each key.
use axum::{extract::Request, middleware::Next, response::Response};
use std::collections::HashSet;
pub async fn first_values(mut request: Request, next: Next) -> Response {
    if let Some(query) = request.uri().query() {
        let mut seen = HashSet::new();
        let mut duplicate = false;
        let pairs: Vec<_> = form_urlencoded::parse(query.as_bytes())
            .filter(|(key, _)| {
                if seen.insert(key.to_string()) {
                    true
                } else {
                    duplicate = true;
                    false
                }
            })
            .collect();
        if duplicate {
            let query = form_urlencoded::Serializer::new(String::new())
                .extend_pairs(pairs)
                .finish();
            let mut parts = request.uri().clone().into_parts();
            if let Ok(path) = format!("{}?{query}", request.uri().path()).parse() {
                parts.path_and_query = Some(path);
                if let Ok(uri) = axum::http::Uri::from_parts(parts) {
                    *request.uri_mut() = uri;
                }
            }
        }
    }
    next.run(request).await
}
