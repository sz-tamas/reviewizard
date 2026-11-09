//! Local browser-write guard; no authentication is added.
use axum::{
    Json,
    extract::Request,
    http::{Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
pub async fn same_origin(request: Request, next: Next) -> Response {
    if allowed(&request) {
        next.run(request).await
    } else {
        (
            StatusCode::FORBIDDEN,
            Json(super::errors::ErrorBody {
                error: "cross-site request refused: this API only accepts writes from the reviewizard page itself".into(),
            }),
        )
            .into_response()
    }
}
fn allowed(request: &Request) -> bool {
    if matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    ) {
        return true;
    }
    let headers = request.headers();
    if let Some(site) = headers.get("sec-fetch-site") {
        let Ok(site) = site.to_str() else {
            return false;
        };
        let site = site.trim().to_ascii_lowercase();
        if !site.is_empty() {
            return site == "same-origin" || site == "none";
        }
    }
    if let Some(origin) = headers.get("origin") {
        let Ok(origin) = origin.to_str() else {
            return false;
        };
        if !origin.is_empty() {
            let host = headers
                .get("host")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("");
            return origin == format!("http://{host}") || origin == format!("https://{host}");
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn regression_browser_header_matrix() {
        for (method, site, origin, ok) in [
            ("POST", "same-origin", "http://127.0.0.1:7777", true),
            ("POST", "none", "", true),
            ("POST", "cross-site", "https://evil.example", false),
            ("DELETE", "cross-site", "https://evil.example", false),
            ("POST", "same-site", "http://localhost:3000", false),
            ("POST", "same-origin", "http://localhost:5173", true),
            ("POST", "", "http://127.0.0.1:7777", true),
            ("POST", "", "https://evil.example", false),
            ("POST", "", "", true),
            ("PATCH", "", "", true),
            ("GET", "cross-site", "https://evil.example", true),
            ("HEAD", "cross-site", "https://evil.example", true),
            ("POST", "weird", "", false),
            ("POST", "  Same-Origin ", "", true),
        ] {
            let mut request = Request::builder()
                .method(method)
                .uri("/api/reviews/1/reset")
                .header("host", "127.0.0.1:7777");
            if !site.is_empty() {
                request = request.header("sec-fetch-site", site);
            }
            if !origin.is_empty() {
                request = request.header("origin", origin);
            }
            assert_eq!(
                allowed(&request.body(axum::body::Body::empty()).unwrap()),
                ok,
                "{method} {site} {origin}"
            );
        }
    }
}
