//! Shared timeout and cancellation behavior for Git subprocesses.
use super::errors::ServiceError;
use std::{
    path::Path,
    process::{Output, Stdio},
    time::Duration,
};
use tokio::{io::AsyncWriteExt, process::Command, time::timeout};

pub async fn git_output(
    root: &Path,
    args: &[&str],
    input: Option<&[u8]>,
) -> Result<Output, ServiceError> {
    git_output_with_env(root, args, input, &[]).await
}
pub async fn git_output_with_env(
    root: &Path,
    args: &[&str],
    input: Option<&[u8]>,
    env: &[(&str, &str)],
) -> Result<Output, ServiceError> {
    #[cfg(test)]
    test_trace::record(root, args);
    let mut cmd = Command::new("git");
    cmd.envs(env.iter().copied());
    cmd.arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = cmd.spawn().map_err(ServiceError::internal)?;
    let stdin = child.stdin.take();
    // Write and drain output concurrently: large check-attr batches must not deadlock.
    timeout(Duration::from_secs(30), async move {
        let write = async move {
            if let (Some(mut stdin), Some(bytes)) = (stdin, input) {
                stdin.write_all(bytes).await?;
                stdin.shutdown().await?;
            }
            Ok::<_, std::io::Error>(())
        };
        let (_, output) = tokio::try_join!(write, child.wait_with_output())?;
        Ok::<_, std::io::Error>(output)
    })
    .await
    .map_err(|_| ServiceError::internal("git command timed out"))?
    .map_err(ServiceError::internal)
}
pub fn checked(output: Output, args: &[&str]) -> Result<Vec<u8>, ServiceError> {
    if !output.status.success() {
        return Err(ServiceError::internal(format!(
            "git {}: {}: {}",
            args.join(" "),
            output
                .status
                .code()
                .map(|code| format!("exit status {code}"))
                .unwrap_or_else(|| "process terminated by signal".into()),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(output.stdout)
}
pub async fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>, ServiceError> {
    checked(git_output(root, args, None).await?, args)
}

// Test-only per-repository process tracing, so cache regressions can be measured
// without unsafe global PATH mutation or interference from concurrently running tests.
#[cfg(test)]
pub(crate) mod test_trace {
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
        sync::{Mutex, OnceLock},
    };
    type Calls = HashMap<PathBuf, Vec<Vec<String>>>;
    static CALLS: OnceLock<Mutex<Calls>> = OnceLock::new();
    pub fn record(path: &Path, args: &[&str]) {
        CALLS
            .get_or_init(Default::default)
            .lock()
            .unwrap()
            .entry(path.into())
            .or_default()
            .push(args.iter().map(|s| s.to_string()).collect());
    }
    pub fn take(path: &Path) -> Vec<Vec<String>> {
        CALLS
            .get_or_init(Default::default)
            .lock()
            .unwrap()
            .remove(path)
            .unwrap_or_default()
    }
}
