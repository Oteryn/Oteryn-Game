//! W05a client replay capture: the real domain-40 server-save countdown snapshot delivered at
//! admission, a local negative command-ownership probe (no player reset command exists) and the
//! countdown delta delivered during the planned reset window. The session retains typed deltas, so a
//! delta capture is the server-delivered value encoded with the production encoder (the C09
//! convention). The consumer is `Session::apply_state_delta` via
//! `crates/session/src/server_save_countdown.rs`.
use super::*;
use oteryn_dev_client::SessionEvent;
use oteryn_protocol_oteryn::server_save_countdown as wire;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "W05A_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

fn countdown_observed(revision: u64, notice: Option<(u64, u64, u64, u8)>) -> bool {
    notice.is_some_and(|(base, next, epoch, minutes)| {
        base == revision
            && base.checked_add(1) == Some(next)
            && epoch != 0
            && (1..=5).contains(&minutes)
    })
}

#[test]
fn countdown_capture_requires_the_first_valid_notice() {
    assert!(!countdown_observed(0, None));
    assert!(!countdown_observed(0, Some((1, 2, 9, 5))));
    assert!(!countdown_observed(u64::MAX, Some((u64::MAX, 0, 9, 5))));
    assert!(countdown_observed(0, Some((0, 1, 9, 5))));
}

fn next_observable_window(remaining: i64) -> Option<u64> {
    (1..=5)
        .rev()
        .map(|minutes| minutes * 60_000)
        .find(|opening| remaining > *opening + 5_000)
        .map(|opening| remaining - opening)
        .filter(|wait| *wait <= 65_000)
        .and_then(|wait| u64::try_from(wait).ok())
}
fn packet_status(notice: bool) -> &'static str {
    if notice { "CAPTURED" } else { "FAIL" }
}
#[test]
fn countdown_window_includes_poll_and_ignores_open_minutes() {
    assert_eq!(next_observable_window(344_000), Some(44_000));
    assert_eq!(next_observable_window(59_000), None);
    assert_eq!(next_observable_window(260_000), Some(20_000));
    assert_eq!(next_observable_window(302_000), Some(62_000));
}
#[test]
fn absent_countdown_has_no_capture_verdict() {
    assert_eq!(packet_status(false), "FAIL");
    assert_eq!(packet_status(true), "CAPTURED");
}

fn packet_outcome(notice: bool, observed: &str) -> String {
    format!(
        "e2e_all scenario=W05a.client status={} evidence={observed}",
        packet_status(notice)
    )
}

async fn wait_for_notice<C, T, E>(
    client: &mut C,
    deadline: tokio::time::Instant,
    mut take_notice: impl FnMut(&mut C) -> Option<T>,
    mut service: impl AsyncFnMut(&mut C, Duration) -> Result<(), E>,
) -> Result<Option<T>, E> {
    loop {
        if let Some(notice) = take_notice(client) {
            return Ok(Some(notice));
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Ok(None);
        }
        // Let liveness finish its final slice so queued frames can be drained at the deadline.
        service(client, remaining.min(Duration::from_secs(7))).await?;
    }
}

#[tokio::test(start_paused = true)]
async fn notice_queued_in_last_liveness_interval_is_observed() -> TestResult {
    let start = tokio::time::Instant::now();
    let delivery = start + Duration::from_millis(49_500);
    let deadline = start + Duration::from_secs(54);
    let mut pending = None;
    let notice = wait_for_notice(
        &mut pending,
        deadline,
        Option::take,
        async |pending, interval| {
            let end = tokio::time::Instant::now() + interval;
            if (tokio::time::Instant::now()..end).contains(&delivery) {
                tokio::time::sleep_until(delivery).await;
                *pending = Some(5);
            }
            tokio::time::sleep_until(end).await;
            Ok::<_, std::convert::Infallible>(())
        },
    )
    .await?;
    assert_eq!(notice, Some(5));
    assert_eq!(tokio::time::Instant::now(), deadline);
    Ok(())
}

#[test]
fn absent_reset_window_emits_a_reportable_failure() {
    let line = crate::qualification::e2e_result_line(&packet_outcome(
        false,
        "countdown=no-observable-reset-window",
    ));
    assert_eq!(
        line,
        "e2e_all scenario=W05a.client status=FAIL seconds=0.000 evidence=countdown=no-observable-reset-window"
    );
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<(bool, String)> {
    let mut client = connect_world(clients, 0xfe).await?;
    if !client
        .selected_capabilities()
        .contains(&wire::CAPABILITY_SERVER_SAVE_COUNTDOWN_V1)
    {
        return Err("admission did not select capability 75".into());
    }
    let snapshot = client
        .session_mut()
        .gated_snapshot(wire::STATE_DOMAIN_SERVER_SAVE_COUNTDOWN)
        .ok_or("domain-40 snapshot missing")?
        .clone();
    if snapshot.revision != 0 || snapshot.payload != wire::SERVER_SAVE_COUNTDOWN_SNAPSHOT {
        return Err(format!("unexpected countdown snapshot {snapshot:?}").into());
    }
    // Capability 75 is snapshot-only. Pairing it with core STEP (type 1)
    // deliberately probes local ownership rejection; this is no reset command
    // and must consume neither a command id nor any server sequence.
    let next_command = client.next_command_id();
    let sequence = client.last_server_sequence();
    let position = client.world_spatial().actor_position;
    let countdown_before = client
        .session_mut()
        .gated_snapshot(wire::STATE_DOMAIN_SERVER_SAVE_COUNTDOWN)
        .cloned();
    let refusal = client
        .session_mut()
        .gated_command(wire::CAPABILITY_SERVER_SAVE_COUNTDOWN_V1, 1, &[])
        .await;
    if !matches!(refusal, Err(oteryn_dev_client::SessionError::CapabilityNotSelected { capability })
        if capability == wire::CAPABILITY_SERVER_SAVE_COUNTDOWN_V1)
        || client.next_command_id() != next_command
        || client.last_server_sequence() != sequence
        || client.world_spatial().actor_position != position
        || client
            .session_mut()
            .gated_snapshot(wire::STATE_DOMAIN_SERVER_SAVE_COUNTDOWN)
            .cloned()
            != countdown_before
    {
        return Err(format!("step=local-route-refusal observed={refusal:?}").into());
    }
    evidence(
        "W05a_CHECK step=player-reset-boundary observed=local-ownership-refusal wire_submission=false command-and-sequence=unchanged countdown-and-position=unchanged",
    );

    let root = crate::durability::DurabilityRoot::connect_test_runtime(clients.url)?;
    let planned = root
        .planned_world_reset(WorldId::decode(&clients.world)?)
        .await?;
    // The owner ignores openings before its first serving poll. Wait for an opening
    // after this observation with time for an initial poll, then allow two polls for delivery.
    let next_opening = planned.and_then(|reset| {
        next_observable_window(
            reset
                .reset_at_unix_ms
                .saturating_sub(reset.database_now_unix_ms),
        )
    });
    let wait_ms = next_opening.map_or(7_000, |wait| wait + 10_000);
    let deadline = tokio::time::Instant::now() + Duration::from_millis(wait_ms);
    verdict::Harness::global().bounded_wait(deadline, "reset-countdown");
    let notice = wait_for_notice(
        &mut client,
        deadline,
        |client| {
            client
                .take_events()
                .into_iter()
                .find_map(|event| match event {
                    SessionEvent::ServerSaveCountdown(delta) => Some(delta),
                    _ => None,
                })
        },
        async |client, interval| client.service_liveness(interval).await,
    )
    .await?;
    let notice = match notice {
        Some(delta) => delta,
        None if next_opening.is_some() => {
            return Err(verdict::HarnessError::boxed(
                verdict::Cause::Deadline {
                    phase: "reset-countdown",
                },
                "expected reset countdown was absent at deadline",
            ));
        }
        None => {
            let expected = format!(
                "snapshot_revision={}\ncountdown=absent\n",
                snapshot.revision
            );
            capture("w05a-snapshot.bin", &snapshot.payload);
            capture("w05a-expected.txt", expected.as_bytes());
            return Ok((
                false,
                "snapshot=validated;countdown=no-observable-reset-window".into(),
            ));
        }
    };
    if !countdown_observed(
        snapshot.revision,
        Some((
            notice.base_revision,
            notice.new_revision,
            notice.value.ending_epoch,
            notice.value.minutes_remaining,
        )),
    ) {
        return Err(format!("invalid countdown delta {notice:?}").into());
    }
    capture("w05a-snapshot.bin", &snapshot.payload);
    capture(
        "w05a-countdown.bin",
        &wire::encode_server_save_countdown(notice.value).map_err(|error| format!("{error:?}"))?,
    );
    let expected = format!(
        "snapshot_revision={}\ncountdown=present\nbase_revision={}\nnew_revision={}\nending_epoch={}\nminutes_remaining={}\n",
        snapshot.revision,
        notice.base_revision,
        notice.new_revision,
        notice.value.ending_epoch,
        notice.value.minutes_remaining,
    );
    capture("w05a-expected.txt", expected.as_bytes());
    Ok((true, expected.replace('\n', ";")))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok((notice, observed))) => evidence(&packet_outcome(notice, &observed)),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W05a.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W05a.client status=FAIL evidence=timeout"),
    }
}
