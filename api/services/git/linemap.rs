use super::diff::Hunk;
fn starts(header: &str) -> (i64, i64) {
    let fields: Vec<_> = header.split(' ').collect();
    let n = |s: &str| {
        s.trim_start_matches(['-', '+'])
            .split(',')
            .next()
            .unwrap_or("")
            .parse()
            .unwrap_or(0)
    };
    if fields.len() < 3 {
        (0, 0)
    } else {
        (n(fields[1]), n(fields[2]))
    }
}
pub fn map_old_line(hunks: &[Hunk], old: i64) -> Option<i64> {
    let mut offset = 0;
    for h in hunks {
        let (mut o, mut n) = starts(&h.header);
        if old < o {
            return old.checked_add(offset);
        }
        for line in &h.lines {
            match line.kind {
                "context" => {
                    if o == old {
                        return Some(n);
                    }
                    o += 1;
                    n += 1;
                }
                "del" => {
                    if o == old {
                        return None;
                    }
                    o += 1;
                }
                "add" => n += 1,
                _ => {}
            }
        }
        offset = n - o;
    }
    old.checked_add(offset)
}
pub fn old_extent(hunks: &[Hunk]) -> i64 {
    hunks
        .iter()
        .map(|h| {
            starts(&h.header).0
                + h.lines
                    .iter()
                    .filter(|l| matches!(l.kind, "context" | "del"))
                    .count() as i64
                - 1
        })
        .max()
        .unwrap_or(0)
        .max(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn regression_hunk_headers_and_line_mapping() {
        for (header, old, new) in [
            ("@@ -12,7 +12,9 @@ func normal() {", 12, 12),
            ("@@ -40,6 +42,8 @@ def foo(x: int) -> str:", 40, 42),
            ("@@ -100,4 +100,4 @@ fn f() -> Result<T, E> {", 100, 100),
            ("@@ -5,2 +5,3 @@ total += 1", 5, 5),
            ("@@ -10,3 +10,4 @@ if x == -1 {", 10, 10),
            ("@@ -1 +1 @@", 1, 1),
            ("@@ -20,5 +30,6 @@", 20, 30),
        ] {
            assert_eq!(starts(header), (old, new));
        }
        let files = crate::services::git::diff::parse(
            "diff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -1,3 +1,5 @@\n+new1\n+new2\n a\n b\n c\n",
        )
        .unwrap();
        for (old, new) in [(1, 3), (2, 4), (3, 5), (10, 12)] {
            assert_eq!(map_old_line(&files[0].hunks, old), Some(new));
        }
        assert_eq!(old_extent(&files[0].hunks), 3);
        let files = crate::services::git::diff::parse(
            "diff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -1,3 +1,3 @@\n a\n-old\n+new\n c\n",
        )
        .unwrap();
        assert_eq!(map_old_line(&files[0].hunks, 2), None);
        assert_eq!(map_old_line(&files[0].hunks, 3), Some(3));
    }
}
