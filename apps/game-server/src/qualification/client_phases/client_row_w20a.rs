//! W20a client replay capture: the natural World clock (domain 18). The join snapshot payload and
//! two applied clock deltas are written for replay. The snapshot retains its original bytes and
//! each typed clock delta is re-encoded with the production codec. `crates/session/src/tests/e2e_w20a.rs` replays them through `Session::admit`
//! and `Session::apply_state_delta` and asserts the server-observed minute, light level and
//! colour listed in the expected file. No clock override or fixture is used.
use super::*;
use oteryn_dev_client::SessionEvent;
use oteryn_protocol_oteryn::world_clock::{
    CAPABILITY_WORLD_LIGHT_V1, STATE_DOMAIN_WORLD_CLOCK, decode_world_clock, encode_world_clock,
};

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "W20A_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

fn clock_progresses(previous: u16, next: u16, color: u8) -> bool {
    const DAY: u32 = 1_440;
    if color != 215 || u32::from(previous) >= DAY || u32::from(next) >= DAY {
        return false;
    }
    let elapsed = (u32::from(next) + DAY - u32::from(previous)) % DAY;
    // Thirty real seconds are twelve Tibian minutes, plus one at the minute boundary.
    (1..=13).contains(&elapsed)
}

#[test]
fn world_clock_rejects_repeated_minute_and_accepts_midnight() {
    assert!(!clock_progresses(1439, 1439, 215));
    assert!(clock_progresses(1439, 0, 215));
    assert!(!clock_progresses(500, 499, 215));
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0x96).await?;
    if !client
        .selected_capabilities()
        .contains(&CAPABILITY_WORLD_LIGHT_V1)
    {
        return Err("step=admission world light capability 19 not selected".into());
    }
    let snapshot = client
        .session_mut()
        .gated_snapshot(STATE_DOMAIN_WORLD_CLOCK)
        .ok_or("step=snapshot domain 18 absent")?;
    let (snapshot_revision, snapshot_payload) = (snapshot.revision, snapshot.payload.clone());
    let first = decode_world_clock(&snapshot_payload)
        .map_err(|error| format!("step=snapshot decode {error:?}"))?;
    let mut expected = format!(
        "snapshot_revision={snapshot_revision}\nsnapshot={}:{}:{}\n",
        first.tibian_minute, first.light_level, first.light_color
    );
    let mut previous = snapshot_revision;
    let mut last = first;
    let mut clocks = std::collections::VecDeque::new();
    for step in 1..=2 {
        let delta =
            tokio::time::timeout(Duration::from_secs(30), async {
                loop {
                    clocks.extend(client.take_events().into_iter().filter_map(
                        |event| match event {
                            SessionEvent::WorldClock(delta) => Some(delta),
                            _ => None,
                        },
                    ));
                    if let Some(delta) = clocks.pop_front() {
                        return Ok::<_, oteryn_dev_client::DevClientError>(delta);
                    }
                    client.service_liveness(Duration::from_millis(50)).await?;
                }
            })
            .await
            .map_err(|_| format!("step=delta{step} timeout"))??;
        let clock = delta.value;
        if delta.base_revision != previous
            || delta.new_revision == previous
            || !clock_progresses(last.tibian_minute, clock.tibian_minute, clock.light_color)
        {
            return Err(format!(
                "step=delta{step} base={} new={} clock={clock:?}",
                delta.base_revision, delta.new_revision
            )
            .into());
        }
        let payload = encode_world_clock(clock)
            .map_err(|error| format!("step=delta{step} encode {error:?}"))?;
        capture(&format!("w20a-delta{step}.bin"), &payload);
        expected.push_str(&format!(
            "delta{step}={}:{}:{}\ndelta{step}_sequence={}\ndelta{step}_base={}\ndelta{step}_new={}\n",
            clock.tibian_minute, clock.light_level, clock.light_color, delta.server_sequence, delta.base_revision, delta.new_revision
        ));
        previous = delta.new_revision;
        last = clock;
    }
    capture("w20a-snapshot.bin", &snapshot_payload);
    capture("w20a-expected.txt", expected.as_bytes());
    let _ = client.session_mut().logout().await;
    Ok(format!(
        "minute={}..{} light={}",
        first.tibian_minute, last.tibian_minute, last.light_level
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients)))
        .await
    {
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
