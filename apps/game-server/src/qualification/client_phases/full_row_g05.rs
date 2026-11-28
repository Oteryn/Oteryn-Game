//! G05 delegates local settings persistence to the native client's app-level test.
use super::*;
use oteryn_dev_client::client_ui::reference_preferences;

const SETTINGS_TEST: &str = concat!(
    "play::feature_local_reference_preferences::settings::sync::e2e_n05::",
    "e2e_n05_settings_export_import_backup_defaults_and_corrupt_file",
);

const DESKTOP_TRANSFER: &str = "bevy_shell::game_ui::options::transfer::tests::transfer_exports_resets_imports_and_restores_the_saved_options";
const DESKTOP_SHORTCUT: &str =
    "bevy_shell::tests::ctrl_k_opens_the_hotkey_options_only_outside_blocked_input";
const TEST_TIMEOUT_SECONDS: u64 = 900;

pub(super) async fn run_packet(_clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::observe(client_settings()).await;
    let (status, detail) = match result {
        Ok(()) => ("PASS", "exact-settings-and-desktop-tests".to_owned()),
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
    if reference_preferences::open_key(&[]) {
        return Err("step=open-key observed=fired-without-input".into());
    }
    evidence("G05_SETTINGS_SHORTCUT_CHECK PASS input=empty");

    for test in [SETTINGS_TEST, DESKTOP_TRANSFER, DESKTOP_SHORTCUT] {
        run_client_test(test).await?;
    }
    Ok(())
}

async fn run_client_test(test: &'static str) -> TestResult {
    // Cargo compilation has its own hard deadline. The watchdog resumes
    // after the bounded subprocess wait.
    verdict::Harness::global().bounded_wait(
        tokio::time::Instant::now() + Duration::from_secs(TEST_TIMEOUT_SECONDS + 10),
        "client-settings-subprocess",
    );
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new("timeout")
            .args([
                "--kill-after=10s",
                &format!("{TEST_TIMEOUT_SECONDS}s"),
                "cargo",
                "+1.100.0",
                "test",
                "--locked",
                "-p",
                "oteryn-client",
                "--lib",
                "--features",
                "bevy-shell",
                "--jobs",
                "2",
                test,
                "--",
                "--exact",
                "--format",
                "pretty",
            ])
            .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
            .output()
    })
    .await??;
    let stdout = String::from_utf8_lossy(&output.stdout);
    print!("{stdout}");
    let passed_line = format!("test {test} ... ok");
    let passed = stdout.lines().any(|line| line == passed_line)
        && stdout
            .lines()
            .any(|line| line.starts_with("test result: ok. 1 passed; 0 failed; 0 ignored;"));
    if !output.status.success() || !passed {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
        return Err(format!(
            "step=client-settings test={test} observed={}",
            output.status
        )
        .into());
    }
    Ok(())
}
