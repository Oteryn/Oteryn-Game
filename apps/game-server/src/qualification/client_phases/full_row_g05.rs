//! G05 delegates local settings persistence to the native client's app-level test.
use super::*;
use oteryn_dev_client::client_ui::reference_preferences;

const SETTINGS_TEST: &str = concat!(
    "play::feature_local_reference_preferences::settings::sync::e2e_n05::",
    "e2e_n05_settings_export_import_backup_defaults_and_corrupt_file",
);

const DESKTOP_TRANSFER: &str = "bevy_shell::game_ui::options::transfer::tests::transfer_exports_resets_imports_and_restores_the_saved_options";
const DESKTOP_SHORTCUT: &str =
    "bevy_shell::owner::tests::ctrl_k_opens_the_hotkey_options_only_outside_blocked_input";
const TEST_TIMEOUT_SECONDS: u64 = 900;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::observe(client_settings(clients)).await;
    let (status, detail) = match result {
        Ok(()) => ("PASS", "exact-settings-and-desktop-relog".to_owned()),
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

async fn client_settings(clients: &SeamClients<'_>) -> TestResult {
    if reference_preferences::open_key(&[]) {
        return Err("step=open-key observed=fired-without-input".into());
    }
    evidence("G05_SETTINGS_SHORTCUT_CHECK PASS input=empty");

    run_client_test(SETTINGS_TEST, None).await?;
    run_client_test(DESKTOP_SHORTCUT, None).await?;

    let output = std::process::Command::new("mktemp")
        .args(["-d", "oteryn-g05.XXXXXXXX"])
        .current_dir(std::env::temp_dir())
        .output()?;
    if !output.status.success() {
        return Err("G05 could not create private preferences directory".into());
    }
    let name = std::str::from_utf8(&output.stdout)?.trim();
    if !name.strip_prefix("oteryn-g05.").is_some_and(|suffix| {
        suffix.len() == 8 && suffix.bytes().all(|byte| byte.is_ascii_alphanumeric())
    }) {
        return Err("G05 received an invalid private directory name".into());
    }
    let root = std::env::temp_dir().join(name);
    let result = desktop_relog(clients, &root).await;
    let cleanup = std::fs::remove_dir_all(&root);
    result?;
    cleanup?;
    Ok(())
}

async fn desktop_relog(clients: &SeamClients<'_>, root: &Path) -> TestResult {
    let character = uuid_text(&clients.characters[1]).replace('-', "");
    let mut first = connect_world(clients, 0x75).await?;
    if tile(first.world_spatial().actor_position)? != TOWN {
        return Err("G05 first admission did not enter the logout-safe town".into());
    }
    client_test_while_admitted(&mut first, root, &character, "transfer").await?;
    let logout = first.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("G05 first logout refused: {logout:?}").into());
    }
    drop(first);
    let mut second = connect_world(clients, 0x76).await?;
    if tile(second.world_spatial().actor_position)? != TOWN {
        return Err("G05 relog did not restore the town position".into());
    }
    client_test_while_admitted(&mut second, root, &character, "reopen").await?;
    let logout = second.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("G05 relog cleanup refused: {logout:?}").into());
    }
    evidence(&format!("G05_SETTINGS_RELOG_CHECK PASS character={character}"));
    Ok(())
}

async fn client_test_while_admitted(
    client: &mut oteryn_dev_client::DevClientSession,
    root: &Path,
    character: &str,
    phase: &str,
) -> TestResult {
    let test = run_client_test(DESKTOP_TRANSFER, Some((root, character, phase)));
    tokio::pin!(test);
    loop {
        let idle = client.service_liveness(Duration::from_secs(1));
        tokio::pin!(idle);
        tokio::select! {
            result = &mut test => {
                idle.await?;
                return result;
            }
            result = &mut idle => result?,
        }
    }
}

async fn run_client_test(
    test: &'static str,
    relog: Option<(&Path, &str, &str)>,
) -> TestResult {
    let relog = relog.map(|(root, character, phase)| {
        (root.to_owned(), character.to_owned(), phase.to_owned())
    });
    // Cargo compilation has its own hard deadline. The watchdog resumes
    // after the bounded subprocess wait.
    verdict::Harness::global().bounded_wait(
        tokio::time::Instant::now() + Duration::from_secs(TEST_TIMEOUT_SECONDS + 10),
        "client-settings-subprocess",
    );
    let output = tokio::task::spawn_blocking(move || {
        let mut command = std::process::Command::new("timeout");
        command.args([
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
            .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."));
        if let Some((root, character, phase)) = relog {
            command.env("OTERYN_G05_SETTINGS_DIR", root)
                .env("OTERYN_G05_CHARACTER", character)
                .env("OTERYN_G05_PHASE", phase);
        }
        command.output()
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
