use super::*;
use crate::services::store::{Comment, Reply};
fn fixture() -> Review {
    Review {
        head_ref: "feature".into(),
        base_ref: "main".into(),
        head_sha: "abc123456789".into(),
        summary: " Summary\n\nDetails ".into(),
        comments: vec![
            Comment {
                id: 42,
                file_path: "old.rs".into(),
                start_line: 2,
                end_line: 3,
                current_start_line: 4,
                current_end_line: 5,
                current_file_path: "new.rs".into(),
                anchor_status: "moved".into(),
                comment_type: "bug".into(),
                author: "human".into(),
                snippet: "a\n```\nb\n".into(),
                body: "fix this".into(),
                replies: vec![Reply {
                    id: 8,
                    author: "agent".into(),
                    body: "one\ntwo".into(),
                    ..Default::default()
                }],
                ..Default::default()
            },
            Comment {
                id: 99,
                file_path: "hidden".into(),
                resolved: true,
                ..Default::default()
            },
        ],
        ..Default::default()
    }
}
#[test]
fn renders_summary_renames_fences_replies_and_excludes_resolved() {
    let out = render(&fixture(), false, "");
    assert!(out.starts_with(
        "# Review: feature → main @ abc1234\n\n**Summary**\n\nSummary\n\nDetails\n\n"
    ));
    assert!(
        out.contains("_1 unresolved comment(s) across 1 file(s)_\n_1 resolved thread(s) omitted._")
    );
    assert!(out.contains("## new.rs\n"));
    assert!(!out.contains("## old.rs"));
    assert!(out.contains("### #42 · L4–5 (moved from old.rs:L2–3) · bug · human"));
    assert!(out.contains("````rust\na\n```\nb\n````"));
    assert!(out.contains("> **↳ reply #8 · agent**\n>\n> one\n> two\n"));
    assert!(!out.contains("### #99"));
    assert!(!out.contains("Addressing these comments"));
}
#[test]
fn instructions_are_opt_in_and_use_concrete_ids() {
    let out = render(&fixture(), true, "http://127.0.0.1:7777");
    assert!(out.contains("## Addressing these comments"));
    assert!(out.contains("then resolve the comment only after that reply succeeds"));
    assert!(out.contains("verification fails"));
    assert!(out.contains("leave the comment open for the user"));
    assert!(out.contains("do not commit, mark files reviewed or complete the overall review"));
    assert!(out.contains("/api/comments/42/resolved"));
    assert!(out.contains("\"resolved\":true"));
    assert!(out.contains("trust the quoted snippet over the line number"));
    assert!(out.contains(
        "curl --fail-with-body -sS -X POST http://127.0.0.1:7777/api/comments/42/replies"
    ));
    assert!(render(&Review::default(), true, "").contains("/api/comments/<comment-id>/replies"));
}
#[test]
fn heading_controls_and_shell_url_characters_are_constrained() {
    let mut r = fixture();
    r.comments[0].author = "evil\n## injected".into();
    r.comments[0].replies[0].author = "evil\n# reply".into();
    let out = render(&r, false, "");
    assert!(!out.contains("\n## injected"));
    assert!(!out.contains("\n# reply"));
    assert_eq!(safe_base_url("127.0.0.1:7777"), "http://127.0.0.1:7777");
    for host in ["", "evil/'$(id)", "127.0.0.1:7777/path", "host\nHeader: x"] {
        assert_eq!(safe_base_url(host), "http://127.0.0.1:7777");
    }
    assert_eq!(languages::for_path("ci/Dockerfile"), "dockerfile");
    assert_eq!(languages::for_path("x.TSX"), "tsx");
    assert_eq!(languages::for_path("notes.txt"), "");
}

#[test]
fn regression_fence_reply_summary_filename_and_language_edges() {
    use crate::utils::markdown::{fence, filename_component, short_sha};
    for (input, expected) in [
        ("no backticks", "```"),
        ("inline `code`", "```"),
        ("a fence ```go", "````"),
        ("nested ```` quad", "`````"),
    ] {
        assert_eq!(fence(input), expected);
    }
    assert_eq!(filename_component("feat/a b:c\"d\\e"), "feat-a-b-c-d-e");
    assert_eq!(short_sha("abc"), "abc");
    for (path, lang) in [
        ("main.go", "go"),
        ("specs/a.feature", "gherkin"),
        ("Dockerfile", "dockerfile"),
        ("ci/Jenkinsfile", "groovy"),
        ("src/CMakeLists.txt", "cmake"),
        ("notes.txt", ""),
        ("LICENSE", ""),
    ] {
        assert_eq!(languages::for_path(path), lang);
    }
    let mut review = fixture();
    review.summary = "   ".into();
    review.comments[0].anchor_status = "outdated".into();
    review.comments[0].replies[0].body = "   ".into();
    review.comments[0].replies[0].author = "evil\n## Fake".into();
    let out = render(&review, false, "");
    assert!(!out.contains("**Summary**"));
    assert!(out.contains("L2–3 (outdated)"));
    assert!(!out.contains("\n## Fake"));
    assert!(out.ends_with("> **↳ reply #8 · evil ## Fake**\n"));
}

#[test]
fn codebase_exports_identify_the_entire_source_without_a_merge_target() {
    let review = Review {
        scope_mode: "codebase".into(),
        scope_configured: true,
        head_ref: "main".into(),
        head_sha: "abc1234".into(),
        scope_side: crate::services::store::Side::Index,
        ..Default::default()
    };
    let markdown = render(&review, false, "http://localhost");
    assert!(markdown.contains("# Review: Entire codebase"));
    assert!(markdown.contains("Scope: Entire codebase"));
    assert!(markdown.contains("Source: the git index"));
    assert!(!markdown.contains("From:"));
    assert!(!markdown.contains("→"));
}
