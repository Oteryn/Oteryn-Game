//! G05 delegates local settings persistence to the native client's app-level test.
use super::*;

const SETTINGS_TEST: &str = "e2e_n05_settings_export_import_backup_defaults_and_corrupt_file";

pub(super) async fn run_packet(_clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::observe(client_settings()).await;
    let (status, detail) = match result {
        Ok(()) => ("PASS", SETTINGS_TEST.to_owned()),
        Err(error) => (
            "FAIL",
            error
                .to_string()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join("-"),
        ),
    };
    evidence(&format!(
        "e2e_all scenario=G05.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

async fn client_settings() -> TestResult {
    let output = tokio::task::spawn_blocking(|| {
        std::process::Command::new("timeout")
            .args([
                "--kill-after=10s",
                "900s",
                "cargo",
                "+1.100.0",
                "test",
                "--locked",
                "-p",
                "oteryn-client",
                "--lib",
                SETTINGS_TEST,
                "--",
                "--format",
                "pretty",
            ])
            .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
            .output()
    })
    .await??;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let passed = stdout.lines().any(|line| {
        line.starts_with("test ") && line.ends_with(&format!("::{SETTINGS_TEST} ... ok"))
    }) && stdout
        .lines()
        .any(|line| line.starts_with("test result: ok. 1 passed; 0 failed; 0 ignored;"));
    if !output.status.success() || !passed {
        eprintln!("{stdout}{}", String::from_utf8_lossy(&output.stderr));
        return Err(format!("step=client-settings observed={}", output.status).into());
    }
    Ok(())
}
