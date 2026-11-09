//! Parse cat-file records by byte length, never by text delimiters inside blobs.
use super::Repo;
use crate::utils::{errors::ServiceError, process};
use std::collections::{HashMap, HashSet};
pub async fn objects(
    repo: &Repo,
    specs: &[String],
) -> Result<HashMap<String, Vec<u8>>, ServiceError> {
    let mut seen = HashSet::new();
    let specs: Vec<_> = specs
        .iter()
        .filter(|s| !s.is_empty() && !s.contains(['\n', '\r']) && seen.insert(s.as_str()))
        .cloned()
        .collect();
    if specs.is_empty() {
        return Ok(HashMap::new());
    }
    let input = specs.join("\n") + "\n";
    let args = ["-c", "core.quotePath=false", "cat-file", "--batch"];
    let bytes = process::checked(
        process::git_output(&repo.path, &args, Some(input.as_bytes())).await?,
        &args,
    )?;
    parse(&bytes, &specs)
}
pub fn parse(data: &[u8], specs: &[String]) -> Result<HashMap<String, Vec<u8>>, ServiceError> {
    let mut pos = 0;
    let mut out = HashMap::new();
    for spec in specs {
        let nl = data
            .get(pos..)
            .and_then(|v| v.iter().position(|b| *b == b'\n'))
            .ok_or_else(|| ServiceError::internal("truncated cat-file header"))?;
        let header = String::from_utf8_lossy(&data[pos..pos + nl]);
        pos += nl + 1;
        let fields: Vec<_> = header.split_whitespace().collect();
        if matches!(fields.last(), Some(&"missing" | &"ambiguous")) {
            continue;
        }
        let size = fields
            .get(2)
            .filter(|_| fields.len() == 3)
            .and_then(|s| s.parse::<usize>().ok())
            .ok_or_else(|| ServiceError::internal("invalid cat-file header"))?;
        let end = pos
            .checked_add(size)
            .filter(|end| *end < data.len() && data[*end] == b'\n')
            .ok_or_else(|| ServiceError::internal("truncated cat-file payload"))?;
        if fields[1] == "blob" {
            out.insert(spec.clone(), data[pos..end].to_vec());
        }
        pos = end + 1;
    }
    Ok(out)
}
