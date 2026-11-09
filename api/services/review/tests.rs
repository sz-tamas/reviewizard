use super::snippet::*;
#[test]
fn line_splitting_and_ambiguous_matches() {
    assert_eq!(split_lines("a\nb\n"), ["a", "b"]);
    assert_eq!(split_lines(""), [""]);
    let lines = split_lines("x\na\nb\nx\na\nb\n");
    let snip = split_lines("a\nb");
    assert_eq!(matches(&lines, &snip), [1, 4]);
    assert!(position(&lines, 1, &snip));
    assert!(!position(&lines, -1, &snip));
    assert!(!position(&lines, i64::MAX, &snip));
}

#[tokio::test]
async fn regression_shared_cache_and_no_cache_git_process_counts() {
    use crate::{
        services::{
            git::Repo,
            review::{DiffCache, annotate},
            store::{Comment, Review},
        },
        utils::process::test_trace,
    };
    let root = std::env::temp_dir().join(format!("rust-review-cache-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(root.clone());
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(args)
            .env(
                "GIT_CONFIG_GLOBAL",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
            )
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
    };
    git(&["init", "-b", "main"]);
    git(&["config", "user.name", "Test"]);
    git(&["config", "user.email", "test@example.invalid"]);
    git(&["config", "commit.gpgsign", "false"]);
    std::fs::write(root.join("a.txt"), "one\ntwo\nthree\n").unwrap();
    std::fs::write(root.join("b.txt"), "uno\ndos\ntres\n").unwrap();
    git(&["add", "."]);
    git(&["commit", "-m", "base"]);
    let old = git(&["rev-parse", "HEAD"]);
    std::fs::write(root.join("a.txt"), "ONE\ntwo\nthree\n").unwrap();
    std::fs::write(root.join("b.txt"), "UNO\ndos\ntres\n").unwrap();
    git(&["add", "."]);
    git(&["commit", "-m", "edit"]);
    let mut r = Review {
        repo_path: root.to_string_lossy().into(),
        head_ref: "main".into(),
        comments: vec![
            Comment {
                file_path: "a.txt".into(),
                start_line: 2,
                end_line: 2,
                snippet: "two".into(),
                commit_sha: old.clone(),
                ..Default::default()
            },
            Comment {
                file_path: "b.txt".into(),
                start_line: 2,
                end_line: 2,
                snippet: "dos".into(),
                commit_sha: old,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let cache = DiffCache::new();
    annotate(&mut r, Some(&[]), Some(&cache)).await;
    let first = test_trace::take(&root);
    assert_eq!(
        first
            .iter()
            .filter(|args| args.iter().any(|a| a == "diff"))
            .count(),
        1
    );
    annotate(&mut r, Some(&[]), Some(&cache)).await;
    let second = test_trace::take(&root);
    assert!(first.len() > second.len());
    assert!(!second.iter().any(|args| args.iter().any(|a| a == "diff")));
    assert!(r.comments.iter().all(|c| c.anchor_status == "current"));
    annotate(&mut r, Some(&[]), None).await;
    let first = test_trace::take(&root);
    annotate(&mut r, Some(&[]), None).await;
    let second = test_trace::take(&root);
    assert_eq!(first.len(), second.len());
    assert!(second.iter().any(|args| args.iter().any(|a| a == "diff")));
    let _repo = Repo { path: root };
}
