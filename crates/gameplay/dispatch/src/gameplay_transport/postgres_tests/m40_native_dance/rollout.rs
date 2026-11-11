//! Isolate the real receipt rollout flag; never mutate another test's environment.
use super::TestResult;
pub(super) fn receipt_process(test: &str) -> TestResult<bool> {
    if std::env::var("OTERYN_STEP_WAIT_RECEIPTS").ok().as_deref() == Some("1") {
        return Ok(false);
    }
    let (_, name) = test.split_once("::").ok_or("native dance test path")?;
    let output = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", name, "--nocapture", "--test-threads=1"])
        .env("OTERYN_STEP_WAIT_RECEIPTS", "1")
        .output()?;
    use std::io::Write;
    std::io::stdout().write_all(&output.stdout)?;
    std::io::stderr().write_all(&output.stderr)?;
    if !output.status.success()
        || !String::from_utf8_lossy(&output.stdout).contains("M40_NATIVE_DANCE sixteen_steps")
    {
        return Err("native dance receipt process did not prove the complete journey".into());
    }
    Ok(true)
}
