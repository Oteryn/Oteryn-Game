//! W05a full-row journey, derived from the matrix/audit and the scheduled-reset owners.
//!
//! 1. Admit the provisioned node-boot character with capability 75; expect the empty
//!    server-save-countdown snapshot at revision zero and retain its session identity.
//! 2. Send an ordinary STEP player command; expect `Accepted`, a `Moved` disposition and
//!    the matching world-spatial delta. This produces player-owned durable state before save.
//! 3. Attempt the invalid capability-75/core-STEP pairing; expect local CapabilityNotSelected
//!    without a wire submission or command/sequence, countdown or position mutation. Control
//!    owns reset; no player reset command exists.
//! 4. Service the native connection through the reset-owner polling interval; when a planned
//!    reset is in its five-minute window, expect a domain-40 delta whose revisions advance by
//!    one, whose epoch stays constant from the first observed minute through minute 1.
//! 5. Expect the planned reset to close the admitted session. A player must not be able to
//!    suppress, move or trigger that reset; a still-live session after the observed countdown
//!    is the refusal/lifecycle failure.
//! 6. Freshly admit after reset and expect a different GameSession plus the walked position,
//!    proving that the character's durable state survived logout/reset/admission. If node boot
//!    has no planned reset in flight, perform an ordinary logout and fresh admission anyway,
//!    re-check the same durable state, and report the reached missing server path rather than
//!    claiming the reset lifecycle passed.
use super::*;
use oteryn_dev_client::{AppliedDelta, DevClientError, DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::server_save_countdown::{
    CAPABILITY_SERVER_SAVE_COUNTDOWN_V1, SERVER_SAVE_COUNTDOWN_MAX_MINUTES,
    SERVER_SAVE_COUNTDOWN_SNAPSHOT, STATE_DOMAIN_SERVER_SAVE_COUNTDOWN, ServerSaveCountdown,
};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(420), verdict::observe(journey(clients)))
            .await;
    let status = match result {
        Ok(Ok(Outcome::ResetObserved)) => "PASS",
        Ok(Ok(Outcome::ResetNotScheduled)) => "FAIL",
        Ok(Err(error)) => {
            evidence(&format!("W05a_GAP step-failed observed={error}"));
            "FAIL"
        }
        Err(_) => {
            evidence("W05a_GAP step=journey observed=420s-timeout");
            "FAIL"
        }
    };
    evidence(&format!(
        "e2e_all scenario=W05a.full status={status} seconds={:.3}",
        started.elapsed().as_secs_f64()
    ));
}

enum Outcome {
    ResetObserved,
    ResetNotScheduled,
}

fn countdowns(
    events: Vec<SessionEvent>,
) -> impl Iterator<Item = AppliedDelta<ServerSaveCountdown>> {
    events.into_iter().filter_map(|event| match event {
        SessionEvent::ServerSaveCountdown(delta) => Some(delta),
        _ => None,
    })
}

fn proves_reset(initial_epoch: u64, ending_epoch: u64, admitted_epoch: u64) -> bool {
    initial_epoch == ending_epoch && admitted_epoch > initial_epoch
}

fn follows(previous: Option<ServerSaveCountdown>, notice: ServerSaveCountdown) -> bool {
    (1..=SERVER_SAVE_COUNTDOWN_MAX_MINUTES).contains(&notice.minutes_remaining)
        && previous.is_none_or(|before| {
            notice.ending_epoch == before.ending_epoch
                && before.minutes_remaining.checked_sub(1) == Some(notice.minutes_remaining)
        })
}

fn closed_transport(error: &DevClientError) -> bool {
    match error {
        DevClientError::Context { source, .. } => closed_transport(source),
        DevClientError::Session(oteryn_dev_client::SessionError::Io(error)) => {
            error.kind() == std::io::ErrorKind::UnexpectedEof
        }
        _ => false,
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<Outcome> {
    let mut client = connect_world(clients, 0xa5).await?;
    let admitted_session = client.join_snapshot().game_session_id;
    if !client
        .selected_capabilities()
        .contains(&CAPABILITY_SERVER_SAVE_COUNTDOWN_V1)
    {
        return Err("step=admission observed=capability-75-not-selected".into());
    }
    let snapshot = client
        .session_mut()
        .gated_snapshot(STATE_DOMAIN_SERVER_SAVE_COUNTDOWN)
        .ok_or("step=admission observed=domain-40-snapshot-missing")?;
    if snapshot.revision != 0 || snapshot.payload != SERVER_SAVE_COUNTDOWN_SNAPSHOT {
        return Err(format!("step=admission observed=countdown-snapshot-{snapshot:?}").into());
    }

    let before = ActorPosition {
        x: 32369,
        y: 32238,
        floor: 7,
    };
    let saved = ActorPosition { y: 32237, ..before };
    if client.world_spatial().actor_position != before {
        return Err(format!(
            "step=walk-start expected={before:?} observed={:?}",
            client.world_spatial().actor_position
        )
        .into());
    }
    let moved = client.step(StepDirection::North).await?;
    if moved.status != CommandStatus::Accepted
        || moved.disposition != StepDisposition::Moved
        || moved
            .world_spatial_delta
            .as_ref()
            .map(|delta| delta.value.actor_position)
            != Some(saved)
        || client.world_spatial().actor_position != saved
    {
        return Err(format!("step=walk expected={saved:?} observed={moved:?}").into());
    }
    evidence(&format!(
        "W05a_CHECK step=walk direction=North reply={moved:?}"
    ));

    // Capability 75 is snapshot-only. Pairing it with core STEP (type 1)
    // deliberately probes local ownership rejection; this is no reset command
    // and must consume neither a command id nor any server sequence.
    let next_command = client.next_command_id();
    let sequence = client.last_server_sequence();
    let position = client.world_spatial().actor_position;
    let countdown_before = client
        .session_mut()
        .gated_snapshot(STATE_DOMAIN_SERVER_SAVE_COUNTDOWN)
        .cloned();
    let refusal = client
        .session_mut()
        .gated_command(CAPABILITY_SERVER_SAVE_COUNTDOWN_V1, 1, &[])
        .await;
    if !matches!(refusal, Err(oteryn_dev_client::SessionError::CapabilityNotSelected { capability })
        if capability == CAPABILITY_SERVER_SAVE_COUNTDOWN_V1)
        || client.next_command_id() != next_command
        || client.last_server_sequence() != sequence
        || client.world_spatial().actor_position != position
        || client
            .session_mut()
            .gated_snapshot(STATE_DOMAIN_SERVER_SAVE_COUNTDOWN)
            .cloned()
            != countdown_before
    {
        return Err(format!("step=local-route-refusal observed={refusal:?}").into());
    }
    evidence(
        "W05a_CHECK step=player-reset-boundary observed=local-ownership-refusal wire_submission=false command-and-sequence=unchanged countdown-and-position=unchanged",
    );

    let first_notice_deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    verdict::Harness::global().bounded_wait(first_notice_deadline, "first-reset-countdown");
    let mut notices = countdowns(client.take_events());
    let first_notice = loop {
        if let Some(delta) = notices.next() {
            break Some(delta);
        }
        let remaining =
            first_notice_deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break None;
        }
        client
            .service_liveness(remaining.min(Duration::from_millis(250)))
            .await?;
        notices = countdowns(client.take_events());
    };
    if let Some(delta) = first_notice {
        let noticed = tokio::time::Instant::now();
        if delta.base_revision.checked_add(1) != Some(delta.new_revision)
            || !follows(None, delta.value)
        {
            return Err(format!("step=countdown observed=invalid-delta-{delta:?}").into());
        }
        let initial_epoch = super::full_row_w13::query(&mut client, 4)
            .await?
            .reset_epoch;
        if initial_epoch != delta.value.ending_epoch {
            return Err(format!(
                "step=countdown-epoch observed={} need=admitted-epoch-{initial_epoch}",
                delta.value.ending_epoch
            )
            .into());
        }
        evidence(&format!("W05a_CHECK step=countdown delta={delta:?}"));
        // The notice says the reset lands within its minute: `minutes_remaining` minutes at most
        // and more than one minute fewer at least. Require a close in that window
        // and corroborate the reset with the epoch after fresh admission.
        let minutes = u64::from(delta.value.minutes_remaining);
        let earliest =
            Duration::from_secs((minutes - 1) * 60).saturating_sub(Duration::from_secs(5));
        let latest = Duration::from_secs(minutes * 60 + 30);
        verdict::Harness::global().bounded_wait(noticed + latest, "reset-countdown");
        let mut revision = delta.new_revision;
        let mut prior = delta.value;
        let deadline = noticed + latest;
        let mut outcome = Ok(());
        let close = loop {
            for delta in notices.by_ref().chain(countdowns(client.take_events())) {
                if delta.base_revision != revision
                    || revision.checked_add(1) != Some(delta.new_revision)
                    || !follows(Some(prior), delta.value)
                {
                    return Err(format!(
                        "step=countdown observed=invalid-next-delta-{delta:?} previous={prior:?}"
                    )
                    .into());
                }
                revision = delta.new_revision;
                prior = delta.value;
                evidence(&format!("W05a_CHECK step=countdown delta={delta:?}"));
            }
            if let Err(error) = outcome {
                if prior.minutes_remaining != 1 || !closed_transport(&error) {
                    return Err(format!("step=reset-close observed={error} last-countdown={prior:?} need=complete-countdown-and-transport-close").into());
                }
                break error;
            }
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(format!(
                    "step=reset-close observed=session-still-live-{}s-after-{minutes}-minute-notice",
                    latest.as_secs()
                ).into());
            }
            outcome = client
                .service_liveness(remaining.min(Duration::from_secs(1)))
                .await;
        };
        let elapsed = noticed.elapsed();
        if elapsed < earliest {
            return Err(format!("step=reset-close observed=closed-after-{}s-before-the-{minutes}-minute-window error={close:?}", elapsed.as_secs()).into());
        }
        evidence(&format!(
            "W05a_CHECK step=reset-close after={}s reply={close:?}",
            elapsed.as_secs()
        ));
        client = connect_world(clients, 0xa6).await?;
        if client.join_snapshot().game_session_id == admitted_session
            || client.world_spatial().actor_position != saved
        {
            return Err(format!("step=post-reset-admission observed=session-{:?}-position-{:?} expected-new-session-position-{saved:?}", client.join_snapshot().game_session_id, client.world_spatial().actor_position).into());
        }
        let admitted_epoch = super::full_row_w13::query(&mut client, 6)
            .await?
            .reset_epoch;
        if !proves_reset(initial_epoch, prior.ending_epoch, admitted_epoch) {
            return Err(format!(
                "step=post-reset-epoch observed={admitted_epoch} need=epoch-after-{}",
                prior.ending_epoch
            )
            .into());
        }
        evidence(&format!(
            "W05a_CHECK step=post-reset-epoch observed={admitted_epoch}"
        ));
        evidence("W05a_CHECK step=post-reset-admission durable-position=retained");
        let cleanup = client.session_mut().logout().await?;
        if cleanup.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Err(format!("step=post-reset-logout observed={cleanup:?}").into());
        }

        return Ok(Outcome::ResetObserved);
    }

    relog_and_check(&mut client, clients, saved).await?;
    evidence(
        "W05a_GAP step=countdown observed=no-domain-40-delta-within-70s need=admission-near-planned-reset-window source=crates/gameplay/dispatch/src/gameplay_transport/server_save_countdown.rs:90",
    );
    Ok(Outcome::ResetNotScheduled)
}

async fn relog_and_check(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    saved: ActorPosition,
) -> TestResult {
    let old = client.join_snapshot().game_session_id;
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout observed={reply:?}").into());
    }
    *client = connect_world(clients, 0xa6).await?;
    if client.join_snapshot().game_session_id == old
        || client.world_spatial().actor_position != saved
    {
        return Err(format!(
            "step=fresh-admission observed=session={:?} position={:?} expected-position={saved:?}",
            client.join_snapshot().game_session_id,
            client.world_spatial().actor_position
        )
        .into());
    }
    evidence("W05a_CHECK step=fresh-admission durable-position=retained");
    let cleanup = client.session_mut().logout().await?;
    if cleanup.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=final-logout observed={cleanup:?}").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_close_and_readmission_on_same_epoch_does_not_prove_reset() {
        assert!(!proves_reset(7, 7, 7));
        assert!(!proves_reset(7, 7, 6));
        assert!(proves_reset(7, 7, 8));
        assert!(!proves_reset(8, 7, 8));
    }

    #[test]
    fn initial_read_retains_each_countdown_in_sequence() {
        let frame = |base_revision, minutes_remaining| {
            SessionEvent::ServerSaveCountdown(AppliedDelta {
                server_sequence: base_revision + 10,
                base_revision,
                new_revision: base_revision + 1,
                value: ServerSaveCountdown {
                    ending_epoch: 7,
                    minutes_remaining,
                },
            })
        };
        let mut buffered = countdowns(vec![frame(0, 5), frame(1, 4)]);
        assert_eq!(
            buffered.next().map(|frame| frame.value.minutes_remaining),
            Some(5)
        );
        assert_eq!(
            buffered.next().map(|frame| frame.value.minutes_remaining),
            Some(4)
        );
        assert_eq!(buffered.next(), None);
    }

    #[test]
    fn review_countdown_requires_every_remaining_minute_of_the_same_epoch() {
        let notice = |ending_epoch, minutes_remaining| ServerSaveCountdown {
            ending_epoch,
            minutes_remaining,
        };
        assert!(follows(None, notice(7, 5)));
        assert!(follows(None, notice(7, 4)));
        assert!(follows(None, notice(7, 1)));
        assert!(!follows(None, notice(7, 6)));
        assert!(follows(Some(notice(7, 5)), notice(7, 4)));
        assert!(!follows(Some(notice(7, 5)), notice(7, 3)));
        assert!(!follows(Some(notice(7, 5)), notice(8, 4)));
        assert!(!follows(Some(notice(7, 1)), notice(7, 0)));
    }
}
