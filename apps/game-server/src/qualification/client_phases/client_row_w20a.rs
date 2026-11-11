//! W20a client replay capture: the natural World clock (domain 18). The join snapshot payload and
//! two consecutive raw clock delta payloads, exactly as the client received them, are written
//! byte for byte. `crates/session/src/tests/e2e_w20a.rs` replays them through `Session::admit`
//! and `Session::apply_state_delta` and asserts the server-observed minute, light level and
//! colour listed in the expected file. No clock override or fixture is used.
use super::*;
use oteryn_protocol_oteryn::world_clock::{
    CAPABILITY_WORLD_LIGHT_V1, STATE_DOMAIN_WORLD_CLOCK, decode_world_clock,
};

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    let mut client = connect_world(clients, 0x96).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    if !client.selected_capabilities().contains(&CAPABILITY_WORLD_LIGHT_V1) {
        return Err("step=admission world light capability 19 not selected".into());
    }
    let snapshot = client
        .session_mut()
        .gated_snapshot(STATE_DOMAIN_WORLD_CLOCK)
        .ok_or("step=snapshot domain 18 absent")?;
    let (snapshot_revision, snapshot_payload) = (snapshot.revision, snapshot.payload.clone());
    let first = decode_world_clock(&snapshot_payload).map_err(|error| format!("step=snapshot decode {error:?}"))?;
    let mut expected = format!(
        "snapshot_revision={snapshot_revision}\nsnapshot={}:{}:{}\n",
        first.tibian_minute, first.light_level, first.light_color
    );
    let mut previous = snapshot_revision;
    let mut last = first;
    for step in 1..=2 {
        let delta = tokio::time::timeout(
            Duration::from_secs(30),
            client.session_mut().read_gated_delta(STATE_DOMAIN_WORLD_CLOCK, 1),
        )
        .await
        .map_err(|_| format!("step=delta{step} timeout"))??;
        let clock = decode_world_clock(&delta.payload).map_err(|error| format!("step=delta{step} decode {error:?}"))?;
        if delta.base_revision != previous || delta.new_revision == previous || clock.light_color != 215 || clock.tibian_minute >= 1_440 {
            return Err(format!("step=delta{step} base={} new={} clock={clock:?}", delta.base_revision, delta.new_revision).into());
        }
        capture(&format!("w20a-delta{step}.bin"), &delta.payload)?;
        expected.push_str(&format!(
            "delta{step}={}:{}:{}\ndelta{step}_sequence={}\ndelta{step}_base={}\ndelta{step}_new={}\n",
            clock.tibian_minute, clock.light_level, clock.light_color, delta.server_sequence, delta.base_revision, delta.new_revision
        ));
        previous = delta.new_revision;
        last = clock;
    }
    capture("w20a-snapshot.bin", &snapshot_payload)?;
    capture("w20a-expected.txt", expected.as_bytes())?;

    Ok(format!("minute={}..{} light={}", first.tibian_minute, last.tibian_minute, last.light_level))

    }).await.unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W20a.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W20a.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=W20a.client status=FAIL evidence=timeout"),
    }
}
