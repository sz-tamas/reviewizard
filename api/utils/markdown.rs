//! Plain inline fields and safe code fences shared by artifact renderers.
pub fn inline_field(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .trim()
        .into()
}
pub fn fence(s: &str) -> String {
    let (mut run, mut longest) = (0, 0);
    for c in s.chars() {
        if c == '`' {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    "`".repeat((longest + 1).max(3))
}
pub fn short_sha(s: &str) -> String {
    s.chars().take(7).collect()
}
pub fn filename_component(s: &str) -> String {
    s.chars()
        .map(|c| {
            if matches!(c, '/' | ' ' | ':' | '"' | '\\') {
                '-'
            } else {
                c
            }
        })
        .collect()
}
