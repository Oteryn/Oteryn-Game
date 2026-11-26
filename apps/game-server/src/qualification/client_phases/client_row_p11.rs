//! P11 client replay capture: the real typed account-achievements page 0 and local codec
//! refusal of malformed query bytes, without a wire submission. The consumer is
//! `Session::account_achievements` in `crates/session/src/account_achievements.rs`.
use super::*;
use oteryn_dev_client::client_runtime::account_achievements::Response;
use oteryn_protocol_oteryn::account_achievements as wire;
use std::os::unix::fs::DirBuilderExt;

const REPLAY_TEST: &str = "tests::e2e_p11::e2e_p11_replays_achievement_page_refusal_and_wrong_page";
const REPLAY_TIMEOUT: Duration = Duration::from_secs(105);

fn verify_replay(page: &[u8], expected: &str) -> TestResult {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let root = std::env::temp_dir().join(format!("oteryn-p11-{}-{nonce}", std::process::id()));
    std::fs::DirBuilder::new().mode(0o700).create(&root)?;
    let result = (|| -> TestResult {
        std::fs::write(root.join("p11-page.bin"), page)?;
        std::fs::write(root.join("p11-expected.txt"), expected)?;
        let output = std::process::Command::new("timeout")
            .args([
                "--kill-after=5",
                &REPLAY_TIMEOUT.as_secs().to_string(),
                "cargo",
                "test",
                "--locked",
                "-p",
                "oteryn-session",
                "--lib",
                REPLAY_TEST,
                "--",
                "--exact",
            ])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .env("E2E_ALL_CAPTURE_DIR", &root)
            .output()?;
        let stdout = std::str::from_utf8(&output.stdout)?;
        let passed = format!("test {REPLAY_TEST} ... ok");
        if !output.status.success() || !stdout.lines().any(|line| line == passed) {
            return Err(format!("P11 session replay did not pass: {}", output.status).into());
        }
        if let Some(publish) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let publish = Path::new(&publish);
            std::fs::write(publish.join("p11-page.bin"), page)?;
            std::fs::write(publish.join("p11-expected.txt"), expected)?;
        }
        Ok(())
    })();
    let cleanup = std::fs::remove_dir_all(root);
    result?;
    cleanup?;
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0x2f).await?;
    let Response::Page(page) = client.session_mut().account_achievements(0).await? else {
        return Err("page query observed=refused".into());
    };
    // The typed core API never emits malformed query bytes. Exercise its codec
    // boundary explicitly; do not label this local check as a server rejection.
    let next_command = client.next_command_id();
    let sequence = client.last_server_sequence();
    if wire::decode_account_achievements_query(&[0x08]).is_ok()
        || client.next_command_id() != next_command
        || client.last_server_sequence() != sequence
    {
        return Err("malformed query was not refused locally without submission".into());
    }
    let reread = client.session_mut().account_achievements(0).await?;
    if reread != Response::Page(page.clone()) {
        return Err(format!("local malformed refusal changed the page: {reread:?}").into());
    }
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("achievement reader logout was refused".into());
    }
    drop(client);

    // The typed core owner has decoded the real reply; encode that exact value,
    // as other typed capture owners do, for the session replay consumer.
    let encoded = wire::encode_account_achievements_result(&page)
        .map_err(|error| format!("page capture encode: {error:?}"))?;
    let rows: Vec<String> = page
        .rows
        .iter()
        .map(|row| format!("{}:{}:{}", row.key, row.grade, row.points))
        .collect();
    let expected = format!(
        "page={}\ntotal_points={}\nfact_count={}\nhas_more={}\nrows={}\nmalformed_query=local-codec-refusal\nmalformed_wire_submission=false\n",
        page.page,
        page.total_points,
        page.fact_count,
        page.has_more,
        rows.join("|")
    );
    let observed = expected.replace('\n', ";");
    verdict::Harness::global().bounded_wait(
        tokio::time::Instant::now() + REPLAY_TIMEOUT,
        "p11-session-replay",
    );
    tokio::task::spawn_blocking(move || {
        verify_replay(&encoded, &expected).map_err(|error| error.to_string())
    })
    .await??;
    evidence(&format!(
        "P11_CHECK replay={REPLAY_TEST} PASS capture=current-run"
    ));
    Ok(observed)
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P11.client status=PASS evidence=live-page-session-replay;{expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P11.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P11.client status=FAIL evidence=timeout"),
    }
}
