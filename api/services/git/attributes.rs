//! Generated-file defaults and .gitattributes overrides.
use super::{Repo, diff::FileDiff};
use crate::utils::process;
use std::collections::HashMap;
const NAMES: &[&str] = &[
    "bun.lock",
    "bun.lockb",
    "package-lock.json",
    "npm-shrinkwrap.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "deno.lock",
    "go.sum",
    "Cargo.lock",
    "composer.lock",
    "Gemfile.lock",
    "poetry.lock",
    "Pipfile.lock",
    "uv.lock",
    "packages.lock.json",
    "pubspec.lock",
    "flake.lock",
    "mix.lock",
    "gradle.lockfile",
];
const DIRS: &[&str] = &[
    "node_modules",
    "vendor",
    "dist",
    "__snapshots__",
    "__pycache__",
];
const SUFFIXES: &[&str] = &[
    ".min.js",
    ".min.css",
    ".js.map",
    ".css.map",
    ".pb.go",
    ".pb.gw.go",
    ".pb.cc",
    ".pb.h",
    "_pb2.py",
    "_pb2_grpc.py",
    ".g.dart",
    ".freezed.dart",
    ".designer.cs",
    "_generated.go",
    ".generated.ts",
    ".snap",
];

fn generated(path: &str) -> bool {
    let base = path.rsplit('/').next().unwrap_or(path);
    NAMES.contains(&base)
        || path
            .rsplit_once('/')
            .is_some_and(|(dirs, _)| dirs.split('/').any(|d| DIRS.contains(&d)))
        || SUFFIXES.iter().any(|suffix| base.ends_with(suffix))
        || base.starts_with("zz_generated.")
}
pub async fn mark(repo: &Repo, files: &mut [FileDiff]) {
    for file in files.iter_mut() {
        file.generated = generated(file.path());
    }
    if files.is_empty() {
        return;
    }
    let mut input = Vec::new();
    let mut indices: HashMap<String, Vec<usize>> = HashMap::new();
    for (index, file) in files.iter().enumerate() {
        input.extend_from_slice(file.path().as_bytes());
        input.push(0);
        indices.entry(file.path().into()).or_default().push(index);
    }
    let args = [
        "check-attr",
        "-z",
        "--stdin",
        "linguist-generated",
        "linguist-language",
    ];
    let Ok(output) = process::git_output(&repo.path, &args, Some(&input)).await else {
        return;
    };
    let Ok(bytes) = process::checked(output, &args) else {
        return;
    };
    let text = String::from_utf8_lossy(&bytes);
    let fields: Vec<_> = text.trim_end_matches('\0').split('\0').collect();
    for triple in fields.chunks_exact(3) {
        let Some(matches) = indices.get(triple[0]) else {
            continue;
        };
        for &index in matches {
            let file = &mut files[index];
            match (triple[1], triple[2]) {
                ("linguist-generated", "set" | "true") => file.generated = true,
                ("linguist-generated", "unset" | "false") => file.generated = false,
                ("linguist-language", value)
                    if !["unspecified", "set", "unset"].contains(&value) =>
                {
                    file.language = value.into()
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn regression_generated_name_table() {
        for path in [
            "bun.lock",
            "web/bun.lock",
            "go.sum",
            "web/dist/index.js",
            "node_modules/pkg/index.js",
            "internal/vendor/lib.go",
            "src/__snapshots__/a.test.js",
            "static/app.min.js",
            "static/app.js.map",
            "api/service.pb.go",
            "api/service_pb2.py",
            "model.freezed.dart",
            "apis/zz_generated.deepcopy.go",
            "src/__tests__/a.test.js.snap",
        ] {
            assert!(generated(path), "{path}");
        }
        for path in [
            "go.mod",
            "internal/api/api.go",
            "web/src/App.tsx",
            "docs/distributed.md",
            "vendors.txt",
            "lock.go",
            "src/snapshot.ts",
            "distribution/handler.go",
            "web/src/components/map.ts",
        ] {
            assert!(!generated(path), "{path}");
        }
    }
}
