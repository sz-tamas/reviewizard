//! Application command-line configuration.
use clap::Parser;
use std::path::PathBuf;
#[derive(Debug, Parser)]
#[command(name="reviewizard", version=env!("REVIEWIZARD_VERSION"),about="Local Git review API (Rust backend)")]
pub struct Config {
    #[arg(long, default_value = ".", allow_hyphen_values = true)]
    pub root: PathBuf,
    #[arg(long, default_value_t = 7777)]
    pub port: u16,
    /// Defaults to ~/.reviewizard; existing data is backed up before first Rust use.
    #[arg(long)]
    pub data_dir: Option<PathBuf>,
    /// Prune stale draft reviews on startup; zero/negative disables pruning.
    #[arg(long, default_value_t = 30, allow_hyphen_values = true)]
    pub retention_days: i64,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flags_and_disabled_retention() {
        let c = Config::try_parse_from([
            "reviewizard",
            "--retention-days",
            "-1",
            "--port",
            "0",
            "--data-dir",
            "~/reviews",
        ])
        .unwrap();
        assert_eq!(c.retention_days, -1);
        assert_eq!(c.port, 0);
        assert_eq!(c.data_dir.unwrap(), PathBuf::from("~/reviews"));
        assert!(Config::try_parse_from(["app", "--port", "65536"]).is_err());
    }
}
