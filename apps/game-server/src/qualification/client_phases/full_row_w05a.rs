//! W05a full-row journey, derived from the matrix/audit and the scheduled-reset owners.
//!
//! 1. Admit the provisioned node-boot character with capability 75; expect the empty
//!    server-save-countdown snapshot at revision zero and retain its session identity.
//! 2. Send an ordinary STEP player command; expect `Accepted`, a `Moved` disposition and
//!    the matching world-spatial delta. This produces player-owned durable state before save.
//! 3. Verify capability 75 registers a countdown domain and no player command.
//! 4. Service the native connection through the reset-owner polling interval; when a planned
//!    reset is in its five-minute window, expect a domain-40 delta whose revisions advance by
//!    one, whose epoch is nonzero and whose remaining minute is in 5..=1.
//! 5. Expect the planned reset to close the admitted session. A player must not be able to
//!    suppress, move or trigger that reset; a still-live session after the observed countdown
//!    is the refusal/lifecycle failure.
//! 6. Freshly admit after reset and expect a different GameSession plus the walked position,
//!    proving that the character's durable state survived logout/reset/admission. If node boot
//!    has no planned reset in flight, perform an ordinary logout and fresh admission anyway,
//!    re-check the same durable state, and report the reached missing server path rather than
//!    claiming the reset lifecycle passed.
use super::*;
use oteryn_dev_client::{AppliedDelta, DevClientSession, SessionError, SessionEvent};
use oteryn_protocol_oteryn::server_save_countdown::{
    CAPABILITY_SERVER_SAVE_COUNTDOWN_V1, SERVER_SAVE_COUNTDOWN_SNAPSHOT,
    STATE_DOMAIN_SERVER_SAVE_COUNTDOWN, ServerSaveCountdown,
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
            evidence("W05a_GAP step=journey observed=380s-timeout");
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

    let before = client.world_spatial().actor_position;
    let (direction, moved) = move_once(&mut client).await?;
    let saved = client.world_spatial().actor_position;
    if saved == before || moved.world_spatial_delta.is_none() {
        return Err(format!("step=walk observed={moved:?}").into());
    }
    evidence(&format!(
        "W05a_CHECK step=walk direction={direction:?} reply={moved:?}"
    ));

    if !countdown_has_read_only_registration(oteryn_protocol_oteryn::capability_tables::GATED) {
        return Err("step=player-reset-boundary observed=countdown-registry-not-read-only".into());
    }
    evidence(
        "W05a_CHECK step=player-reset-boundary observed=registered-countdown-domain-has-no-player-command",
    );

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

    let mut notice = latest_countdown(client.take_events());
    let mut close = Ok(());
    if notice.is_none() {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(7);
        while notice.is_none() && tokio::time::Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            close = client
                .session_mut()
                .service_liveness(remaining.min(Duration::from_millis(100)))
                .await;
            (notice, close) = countdown_poll(client.take_events(), close)?;
            if close.is_err() {
                break;
            }
        }
    }
    if let Some(delta) = notice {
        if delta.base_revision.checked_add(1) != Some(delta.new_revision)
            || delta.value.ending_epoch == 0
            || !(1..=5).contains(&delta.value.minutes_remaining)
        {
            return Err(format!("step=countdown observed=invalid-delta-{delta:?}").into());
        }
        evidence(&format!("W05a_CHECK step=countdown delta={delta:?}"));
        // The notice says the reset lands within its minute: `minutes_remaining` minutes at most
        // and more than one minute fewer at least. Only a close inside that window is the reset;
        // an earlier disconnect is some other failure.
        let noticed = tokio::time::Instant::now();
        let minutes = u64::from(delta.value.minutes_remaining);
        let earliest =
            Duration::from_secs((minutes - 1) * 60).saturating_sub(Duration::from_secs(5));
        let latest = Duration::from_secs(minutes * 60 + 30);
        verdict::Harness::global().bounded_wait(noticed + latest, "reset-countdown");
        while close.is_ok() {
            let remaining = latest.saturating_sub(noticed.elapsed());
            if remaining.is_zero() {
                break;
            }
            close = client
                .session_mut()
                .service_liveness(remaining.min(Duration::from_millis(100)))
                .await;
            client.take_events();
        }
        let elapsed = noticed.elapsed();
        if !observed_reset_close(&close) {
            return Err(format!(
                "step=reset-close observed={close:?} after-window-budget={}s minutes-remaining={minutes}",
                latest.as_secs()
            )
            .into());
        }
        if !(earliest..=latest).contains(&elapsed) {
            return Err(format!("step=reset-close observed=closed-after-{}s-outside-the-{minutes}-minute-window error={close:?}", elapsed.as_secs()).into());
        }
        evidence(&format!(
            "W05a_CHECK step=reset-close after={}s reply={close:?}",
            elapsed.as_secs()
        ));
        client = connect_world(clients, 0xa6).await?;
        verify_reset_loss(
            clients,
            *admitted_session.as_bytes(),
            delta.value.ending_epoch,
        )
        .await?;
        let active = client
            .session_mut()
            .world_map_snapshot()
            .map(|map| map.header.reset_epoch);
        if !reset_epoch_advanced(delta.value.ending_epoch, active) {
            return Err(format!(
                "step=reset-epoch observed={active:?} expected-successor-of={}",
                delta.value.ending_epoch
            )
            .into());
        }
        if client.join_snapshot().game_session_id == admitted_session
            || client.world_spatial().actor_position != saved
        {
            return Err(format!("step=post-reset-admission observed=session-{:?}-position-{:?} expected-new-session-position-{saved:?}", client.join_snapshot().game_session_id, client.world_spatial().actor_position).into());
        }
        evidence("W05a_CHECK step=post-reset-admission durable-position=retained");
        let cleanup = client.session_mut().logout().await?;
        if cleanup.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Err(format!("step=post-reset-logout observed={cleanup:?}").into());
        }
        return Ok(Outcome::ResetObserved);
    }

    relog_and_check(&mut client, clients, saved).await?;
    evidence("W05a_GAP step=countdown observed=no-domain-40-delta-after-owner-poll");
    Ok(Outcome::ResetNotScheduled)
}

async fn move_once(
    client: &mut DevClientSession,
) -> TestResult<(StepDirection, oteryn_dev_client::StepOutcome)> {
    for direction in [
        StepDirection::North,
        StepDirection::East,
        StepDirection::South,
        StepDirection::West,
    ] {
        let outcome = client.step_retrying(direction).await?;
        if outcome.status == CommandStatus::Accepted
            && outcome.disposition == StepDisposition::Moved
        {
            return Ok((direction, outcome));
        }
    }
    Err("step=walk observed=all-four-player-step-commands-blocked".into())
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

async fn verify_reset_loss(
    clients: &SeamClients<'_>,
    session: [u8; 16],
    ending_epoch: u64,
) -> TestResult {
    use crate::foundation::{ControlLossCauseV1, ControlLossEpochRefV1, GameSessionId};
    let root = crate::durability::DurabilityRoot::connect_test_runtime(clients.url)
        .map_err(|_| "step=reset-close observed=durability-root-unavailable")?;
    root.await_ready()
        .await
        .map_err(|_| "step=reset-close observed=durability-root-not-ready")?;
    let session = GameSessionId::decode(&session)?;
    // This journey starts with a fresh admission and never attempts an earlier loss/reconnect.
    let first_loss =
        ControlLossEpochRefV1::new(1).map_err(|error| format!("first loss epoch: {error:?}"))?;
    let loss = crate::durability::fresh_admission::FreshAdmissionStore::from_root(root)
        .owning_loss(session, first_loss)
        .await
        .map_err(|error| format!("step=reset-close observed=loss-read-{error:?}"))?;
    let cause = loss.map(|(operation, _)| operation.observation.cause);
    let world = WorldId::decode(&clients.world)?;
    if !matches!(cause, Some(ControlLossCauseV1::PlannedWorldReset { world_id, reset_epoch, .. })
        if world_id == world && reset_epoch == ending_epoch)
    {
        return Err(format!("step=reset-close observed=loss-cause-{cause:?} expected=planned-world-reset-{ending_epoch}").into());
    }
    Ok(())
}

fn reset_epoch_advanced(ending: u64, active: Option<u64>) -> bool {
    active.is_some() && ending.checked_add(1) == active
}

type Countdown = AppliedDelta<ServerSaveCountdown>;
type ResetRead = Result<(), SessionError>;

fn latest_countdown(events: Vec<SessionEvent>) -> Option<Countdown> {
    events.into_iter().rev().find_map(|event| match event {
        SessionEvent::ServerSaveCountdown(delta) => Some(delta),
        _ => None,
    })
}

fn countdown_poll(
    events: Vec<SessionEvent>,
    read: ResetRead,
) -> Result<(Option<Countdown>, ResetRead), SessionError> {
    let notice = latest_countdown(events);
    if notice.is_none() {
        read?;
        return Ok((None, Ok(())));
    }
    Ok((notice, read))
}

fn countdown_has_read_only_registration(routes: &[(u32, &[u32], &[u32])]) -> bool {
    routes.iter().any(|(id, commands, domains)| {
        *id == CAPABILITY_SERVER_SAVE_COUNTDOWN_V1
            && commands.is_empty()
            && domains.contains(&STATE_DOMAIN_SERVER_SAVE_COUNTDOWN)
    })
}

fn observed_reset_close(result: &Result<(), oteryn_dev_client::SessionError>) -> bool {
    matches!(result, Err(oteryn_dev_client::SessionError::Io(error)) if error.kind() == std::io::ErrorKind::UnexpectedEof)
}

#[cfg(test)]
mod review_regressions {
    use super::*;
    use oteryn_dev_client::SessionError;

    #[test]
    fn reset_close_requires_eof_and_epoch_advance() {
        assert!(!reset_epoch_advanced(5, Some(5)));
        assert!(!reset_epoch_advanced(5, None));
        assert!(reset_epoch_advanced(5, Some(6)));
        assert!(!observed_reset_close(&Err(SessionError::Timeout(
            "server silence"
        ))));
        assert!(!observed_reset_close(&Err(SessionError::SessionUnusable)));
        assert!(observed_reset_close(&Err(SessionError::Io(
            std::io::ErrorKind::UnexpectedEof.into()
        ))));
    }
}

#[cfg(test)]
mod delta_review_tests {
    use super::*;
    #[test]
    fn countdown_already_received_survives_eof_in_the_same_poll() -> TestResult {
        let notice = AppliedDelta {
            server_sequence: 5,
            base_revision: 0,
            new_revision: 1,
            value: ServerSaveCountdown {
                ending_epoch: 7,
                minutes_remaining: 1,
            },
        };
        let events = vec![SessionEvent::ServerSaveCountdown(notice.clone())];
        assert_eq!(latest_countdown(events.clone()), Some(notice.clone()));
        let eof = || Err(SessionError::Io(std::io::ErrorKind::UnexpectedEof.into()));
        let (observed, close) = countdown_poll(events, eof())?;
        assert_eq!(observed, Some(notice));
        assert!(observed_reset_close(&close));
        assert!(countdown_poll(Vec::new(), eof()).is_err());
        Ok(())
    }

    #[test]
    fn unknown_opcode_rejection_does_not_prove_reset_boundary() {
        assert!(!countdown_has_read_only_registration(&[(
            CAPABILITY_SERVER_SAVE_COUNTDOWN_V1,
            &[1],
            &[STATE_DOMAIN_SERVER_SAVE_COUNTDOWN]
        ),]));
        assert!(countdown_has_read_only_registration(
            oteryn_protocol_oteryn::capability_tables::GATED
        ));
    }
}
