//! Release version metadata.
fn main() {
    println!("cargo:rerun-if-changed=api/services/build.rs");
    println!("cargo:rerun-if-env-changed=REVIEWIZARD_VERSION");
    let version = std::env::var("REVIEWIZARD_VERSION").unwrap_or_else(|_| "dev".into());
    assert!(
        !version.contains(['\n', '\r']),
        "release version must be a single line"
    );
    println!("cargo:rustc-env=REVIEWIZARD_VERSION={version}");
}
