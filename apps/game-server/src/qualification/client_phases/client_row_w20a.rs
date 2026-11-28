//! W20a captures the original server frames from one natural World clock session.
use super::*;
use oteryn_dev_client::SessionEvent;
use oteryn_protocol_oteryn::world_clock::{
    CAPABILITY_WORLD_LIGHT_V1, STATE_DOMAIN_WORLD_CLOCK, decode_world_clock,
};

// At constant light, the pacer only publishes on the next Tibian hour (150 real seconds).
const CLOCK_DELTA_WAIT: Duration =
    Duration::from_millis(60 * crate::world_clock::TIBIAN_MINUTE_MS + 15_000);
const JOURNEY_WAIT: Duration =
    Duration::from_millis(2 * CLOCK_DELTA_WAIT.as_millis() as u64 + 30_000);

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    use std::io::Write;

    let Some(root) = std::env::var_os("NODE_BOOT_W20A_CAPTURE_DIR")
        .or_else(|| std::env::var_os("E2E_ALL_CAPTURE_DIR"))
    else {
        return Err("step=capture directory missing".into());
    };
    std::fs::File::create_new(Path::new(&root).join(name))?.write_all(bytes)?;
    Ok(())
}

fn advances_revision(previous: u64, base: u64, next: u64) -> bool {
    base == previous && next > previous
}

// Independent expectations from WORLD-INTERACTION-0 §11.3.
fn expected_light(minute: u16) -> u8 {
    match minute {
        360..480 => (40 + 7 * ((minute - 360) / 4 + 1)).min(250) as u8,
        480..1050 => 250,
        1050..1170 => (250_u16.saturating_sub(7 * ((minute - 1050) / 4 + 1))).max(40) as u8,
        _ => 40,
    }
}

fn advances_minute(previous: u16, next: u16) -> bool {
    // A publish interval is at most one game hour, with the existing 15-second
    // receive allowance. Midnight wraps in the same interval.
    (1..=66).contains(&((next + 1440 - previous) % 1440))
}

async fn journey(
    clients: &SeamClients<'_>,
) -> TestResult<(oteryn_dev_client::DevClientSession, String, String)> {
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
    if first.light_color != 215
        || first.tibian_minute >= 1_440
        || first.light_level != expected_light(first.tibian_minute)
    {
        return Err(format!("step=snapshot clock={first:?}").into());
    }
    let mut expected = format!(
        "snapshot_revision={snapshot_revision}\nsnapshot={}:{}:{}\n",
        first.tibian_minute, first.light_level, first.light_color
    );
    let mut previous = snapshot_revision;
    let mut last = first;
    let mut pending = std::collections::VecDeque::new();
    for step in 1..=2 {
        let deadline = tokio::time::Instant::now() + CLOCK_DELTA_WAIT;
        verdict::Harness::global().bounded_wait(deadline, "world-clock-delta");
        let delta = loop {
            if let Some(delta) = pending.pop_front() {
                break delta;
            }
            let remaining = deadline
                .checked_duration_since(tokio::time::Instant::now())
                .ok_or_else(|| format!("step=delta{step} timeout"))?;
            pending.extend(
                client
                    .wait_for_events(remaining)
                    .await?
                    .into_iter()
                    .filter_map(|event| match event {
                        SessionEvent::WorldClock(delta) => Some(delta),
                        _ => None,
                    }),
            );
        };
        let clock = delta.value;
        if !advances_revision(previous, delta.base_revision, delta.new_revision)
            || clock.light_color != 215
            || clock.tibian_minute >= 1_440
            || !advances_minute(last.tibian_minute, clock.tibian_minute)
            || clock.light_level != expected_light(clock.tibian_minute)
        {
            return Err(format!(
                "step=delta{step} base={} new={} clock={clock:?}",
                delta.base_revision, delta.new_revision
            )
            .into());
        }
        expected.push_str(&format!(
            "delta{step}={}:{}:{}\ndelta{step}_sequence={}\ndelta{step}_base={}\ndelta{step}_new={}\n",
            clock.tibian_minute, clock.light_level, clock.light_color, delta.server_sequence, delta.base_revision, delta.new_revision
        ));
        previous = delta.new_revision;
        last = clock;
    }

    Ok((
        client,
        expected,
        format!(
            "minute={}..{} light={}",
            first.tibian_minute, last.tibian_minute, last.light_level
        ),
    ))
}

async fn capture_journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let (result, transcript) =
        oteryn_dev_client::native_session::capture_inbound_frames(journey(clients)).await;
    let (mut client, expected, observed) = result?;
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout observed={logout:?} need=Accepted").into());
    }
    capture("w20a-inbound.bin", &transcript)?;
    capture("w20a-expected.txt", expected.as_bytes())?;
    Ok(observed)
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    evidence("W20A_CAPTURE_STARTED");
    match verdict::indexed_timeout(JOURNEY_WAIT, verdict::observe(capture_journey(clients))).await {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_oracle_rejects_backward_and_oversized_steps_and_checks_transitions() {
        assert!(!advances_minute(1210, 1209));
        assert!(!advances_minute(1200, 1267));
        assert!(!advances_minute(1200, 1200));
        assert!(advances_minute(1439, 0));
        for (minute, level) in [
            (359, 40),
            (360, 47),
            (480, 250),
            (1049, 250),
            (1050, 243),
            (1170, 40),
        ] {
            assert_eq!(expected_light(minute), level);
        }
    }

    #[test]
    fn journey_regression_rejects_reversed_capture_revisions() {
        assert!(advances_revision(7, 7, 8));
        assert!(!advances_revision(7, 6, 8));
        assert!(!advances_revision(7, 7, 7));
        assert!(!advances_revision(7, 7, 6));
    }
}
