//! Small, read-only probes of a registered repository root.
use crate::{services::store::Project, utils::process};
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::{self, Read},
    path::Path,
};

const MAX_FILE_BYTES: u64 = 64 * 1024;
const MAX_ROOT_ENTRIES: usize = 4096;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMetadata {
    readme: Option<String>,
    license: Option<String>,
    branch: Option<String>,
    languages: Vec<Language>,
}

#[derive(Serialize)]
struct Language {
    name: &'static str,
    version: Option<String>,
}

pub async fn collect(project: &Project) -> Option<ProjectMetadata> {
    let logical = project.path.clone();
    let canonical = project.canonical_path.clone();
    let (root, mut metadata) = tokio::task::spawn_blocking(move || {
        let root = Path::new(&logical).canonicalize().ok()?;
        if root != Path::new(&canonical) {
            return None;
        }
        let metadata = scan_root(&root).ok()?;
        Some((root, metadata))
    })
    .await
    .ok()??;
    // symbolic-ref also works for unborn branches and linked worktrees.
    let output = process::git_output_with_env(
        &root,
        &["--git-dir=.git", "symbolic-ref", "--quiet", "HEAD"],
        None,
        &[("GIT_OPTIONAL_LOCKS", "0")],
    )
    .await
    .ok()?;
    if output.status.success() {
        metadata.branch = Some(
            String::from_utf8_lossy(&output.stdout)
                .trim_end()
                .strip_prefix("refs/heads/")?
                .to_owned(),
        );
    } else if output.status.code() != Some(1) {
        return None;
    }
    Some(metadata)
}

fn scan_root(root: &Path) -> io::Result<ProjectMetadata> {
    let mut readme = None;
    let mut license = None;
    let mut files = BTreeMap::new();
    let mut languages = BTreeMap::new();
    for (index, entry) in fs::read_dir(root)?.enumerate() {
        if index >= MAX_ROOT_ENTRIES {
            return Err(io::Error::other("too many root entries"));
        }
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let lower = name.to_ascii_lowercase();
        let stem = lower.split('.').next().unwrap_or_default();
        if stem == "readme" && readme.as_ref().is_none_or(|previous| &name < previous) {
            readme = Some(name.clone());
        }
        if matches!(stem, "license" | "licence" | "copying")
            && license.as_ref().is_none_or(|previous| &name < previous)
        {
            license = Some(name.clone());
        }
        if let Some(language) = match lower.rsplit_once('.').map(|(_, extension)| extension) {
            Some("rs") => Some("Rust"),
            Some("ts" | "tsx") => Some("TypeScript"),
            Some("js" | "jsx" | "mjs" | "cjs") => Some("JavaScript"),
            Some("py") => Some("Python"),
            Some("go") => Some("Go"),
            Some("rb") => Some("Ruby"),
            Some("java") => Some("Java"),
            Some("cs") => Some("C#"),
            Some("c" | "h") => Some("C"),
            Some("cpp" | "cc" | "hpp") => Some("C++"),
            _ => None,
        } {
            languages.insert(language, None);
        }
        if matches!(
            name.as_str(),
            "Cargo.toml"
                | "rust-toolchain.toml"
                | "rust-toolchain"
                | "package.json"
                | "tsconfig.json"
                | ".node-version"
                | ".nvmrc"
                | "pyproject.toml"
                | ".python-version"
                | "go.mod"
                | "Gemfile"
                | ".ruby-version"
                | "mise.toml"
                | ".mise.toml"
                | ".tool-versions"
        ) {
            files.insert(name, entry.path());
        }
    }
    let read = |name| files.get(name).and_then(|path| read_small(path));
    let package = read("package.json").and_then(|text| serde_json::from_str::<Value>(&text).ok());
    if files.contains_key("package.json") {
        let typescript = package.as_ref().and_then(|p| {
            p.pointer("/devDependencies/typescript")
                .or_else(|| p.pointer("/dependencies/typescript"))?
                .as_str()
                .and_then(version)
        });
        if typescript.is_some() || files.contains_key("tsconfig.json") {
            languages.insert("TypeScript", typescript);
        }
        languages.insert(
            "Node.js",
            read(".node-version")
                .or_else(|| read(".nvmrc"))
                .and_then(|v| version(&v))
                .or_else(|| version(package.as_ref()?.pointer("/engines/node")?.as_str()?)),
        );
    }
    if files.contains_key("Cargo.toml") || languages.contains_key("Rust") {
        languages.insert(
            "Rust",
            read("rust-toolchain.toml")
                .and_then(|text| setting(&text, "toolchain", "channel"))
                .or_else(|| read("rust-toolchain").and_then(|v| version(&v)))
                .or_else(|| setting(&read("Cargo.toml")?, "package", "rust-version")),
        );
    }
    if files.contains_key("pyproject.toml") || files.contains_key(".python-version") {
        languages.insert(
            "Python",
            read(".python-version")
                .and_then(|v| version(&v))
                .or_else(|| setting(&read("pyproject.toml")?, "project", "requires-python")),
        );
    }
    if let Some(text) = read("go.mod") {
        languages.insert(
            "Go",
            text.lines()
                .find_map(|line| version(line.strip_prefix("go ")?)),
        );
    }
    if files.contains_key("Gemfile") || files.contains_key(".ruby-version") {
        languages.insert("Ruby", read(".ruby-version").and_then(|v| version(&v)));
    }
    let mise = read("mise.toml").or_else(|| read(".mise.toml"));
    let tool_versions = read(".tool-versions");
    for (tool, name) in [
        ("rust", "Rust"),
        ("node", "Node.js"),
        ("nodejs", "Node.js"),
        ("python", "Python"),
        ("go", "Go"),
        ("golang", "Go"),
        ("ruby", "Ruby"),
    ] {
        let pinned = mise
            .as_deref()
            .and_then(|text| setting(text, "tools", tool))
            .or_else(|| {
                tool_versions.as_deref()?.lines().find_map(|line| {
                    let mut words = line.split_whitespace();
                    (words.next()? == tool)
                        .then(|| words.next())
                        .flatten()
                        .and_then(version)
                })
            });
        if let Some(pinned) = pinned {
            languages.insert(name, Some(pinned));
        }
    }
    Ok(ProjectMetadata {
        readme,
        license: license.map(|name| {
            read_small(&root.join(&name))
                .as_deref()
                .and_then(license_id)
                .map(str::to_owned)
                .unwrap_or(name)
        }),
        branch: None,
        languages: languages
            .into_iter()
            .map(|(name, version)| Language { name, version })
            .collect(),
    })
}

fn read_small(path: &Path) -> Option<String> {
    if !fs::symlink_metadata(path).ok()?.is_file() {
        return None;
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path).ok()?;
    if !file.metadata().ok()?.is_file() {
        return None;
    }
    let mut text = String::new();
    file.take(MAX_FILE_BYTES + 1)
        .read_to_string(&mut text)
        .ok()?;
    (text.len() as u64 <= MAX_FILE_BYTES).then_some(text)
}

// Read simple quoted scalar declarations. Project scripts and tool managers are never invoked.
fn setting(text: &str, section: &str, key: &str) -> Option<String> {
    let section = format!("[{section}]");
    for line in text
        .lines()
        .map(str::trim)
        .skip_while(|line| *line != section)
        .skip(1)
        .take_while(|line| !line.starts_with('['))
    {
        if let Some((name, value)) = line.split_once('=')
            && name.trim() == key
        {
            let value = value.trim_start();
            let quote = value.chars().next()?;
            if matches!(quote, '"' | '\'') {
                return version(value[1..].split_once(quote)?.0);
            }
        }
    }
    None
}

fn version(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty() && value.len() <= 80 && !value.chars().any(char::is_control))
        .then(|| value.to_owned())
}

fn license_id(text: &str) -> Option<&'static str> {
    let text = text.to_ascii_lowercase();
    [
        ("AGPL-3.0", "gnu affero general public license", "version 3"),
        ("LGPL-3.0", "gnu lesser general public license", "version 3"),
        (
            "LGPL-2.1",
            "gnu lesser general public license",
            "version 2.1",
        ),
        ("GPL-3.0", "gnu general public license", "version 3"),
        ("GPL-2.0", "gnu general public license", "version 2"),
        ("Apache-2.0", "apache license", "version 2.0"),
        ("MPL-2.0", "mozilla public license", "version 2.0"),
        (
            "MIT",
            "permission is hereby granted, free of charge",
            "the software is provided",
        ),
        (
            "ISC",
            "permission to use, copy, modify, and/or distribute",
            "the software is provided",
        ),
        (
            "BSD-3-Clause",
            "redistribution and use in source and binary forms",
            "neither the name",
        ),
        (
            "BSD-2-Clause",
            "redistribution and use in source and binary forms",
            "this list of conditions",
        ),
        (
            "Unlicense",
            "unencumbered software released into the public domain",
            "the software is provided",
        ),
    ]
    .into_iter()
    .find_map(|(id, first, second)| {
        (text.trim().eq_ignore_ascii_case(id) || text.contains(first) && text.contains(second))
            .then_some(id)
    })
}
