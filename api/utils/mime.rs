//! Stable mappings shared by blob services.
pub fn for_path(path: &str) -> String {
    let extension = std::path::Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mime = match extension.as_str() {
        "ico" => "image/x-icon".into(),
        "js" | "mjs" => "text/javascript".into(),
        _ => mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string(),
    };
    if mime.starts_with("text/") {
        format!("{mime}; charset=utf-8")
    } else {
        mime
    }
}
