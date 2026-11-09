//! Explicit home expansion, without changing environment variables or invoking a shell.
use super::errors::ServiceError;
use std::path::{Path, PathBuf};
pub fn home() -> Option<PathBuf> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}
pub fn expand_home(path: &Path, home: Option<&Path>) -> Result<PathBuf, ServiceError> {
    let text = path.to_string_lossy();
    if text == "~" || text.starts_with("~/") || cfg!(windows) && text.starts_with("~\\") {
        let home = home.ok_or_else(|| ServiceError::internal("cannot resolve home directory"))?;
        return Ok(home.join(text.get(2..).unwrap_or("")));
    }
    Ok(path.into())
}
/// Preserve aliases while cleaning lexical dot/parent components.
pub fn absolute_clean(path: &Path) -> std::io::Result<PathBuf> {
    let absolute = std::path::absolute(path)?;
    let mut result = PathBuf::new();
    for component in absolute.components() {
        match component {
            std::path::Component::ParentDir => {
                result.pop();
            }
            std::path::Component::CurDir => {}
            other => result.push(other.as_os_str()),
        }
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn expands_only_explicit_home_prefix() {
        let h = Path::new("home");
        assert_eq!(
            expand_home(Path::new("~/reviews"), Some(h)).unwrap(),
            h.join("reviews")
        );
        assert_eq!(expand_home(Path::new("~"), Some(h)).unwrap(), h);
        assert_eq!(
            expand_home(Path::new("~someone"), None).unwrap(),
            Path::new("~someone")
        );
        assert!(expand_home(Path::new("~/x"), None).is_err());
    }
}
