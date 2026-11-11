//! W05a full-row journey, derived from the matrix/audit and the scheduled-reset owners.
//!
//! 1. Admit the provisioned node-boot character with capability 75; expect the empty
//!    server-save-countdown snapshot at revision zero and retain its session identity.
//! 2. Send an ordinary STEP player command; expect `Accepted`, a `Moved` disposition and
//!    the matching world-spatial delta. This produces player-owned durable state before save.
//! 3. Assert the native local gate refuses a capability-75 command: countdown is receive-only
//!    and no player reset command exists. Require the player position to remain unchanged.
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
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::server_save_countdown::{
    CAPABILITY_SERVER_SAVE_COUNTDOWN_V1, SERVER_SAVE_COUNTDOWN_SNAPSHOT,
    STATE_DOMAIN_SERVER_SAVE_COUNTDOWN,
};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete_with_timeout(Duration::from_secs(420), journey(clients)).await;
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
    let mut client = row_connect_world(clients, 0xa5).await?;
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

    let refusal = client
        .session_mut()
        .gated_command(CAPABILITY_SERVER_SAVE_COUNTDOWN_V1, 1, &[])
        .await;
    if !matches!(&refusal, Err(error) if format!("{error:?}") == "CapabilityNotSelected { capability: 75 }")
        || client.world_spatial().actor_position != saved
    {
        return Err(format!("step=player-reset-local-gate observed={refusal:?}").into());
    }
    evidence(&format!(
        "W05a_CHECK step=player-reset-local-gate reply={refusal:?}"
    ));


    client.take_events();
    client.service_liveness(Duration::from_secs(7)).await?;
    let notice = client
        .take_events()
        .into_iter()
        .find_map(|event| match event {
            SessionEvent::ServerSaveCountdown(delta) => Some(delta),
            _ => None,
        });
    if let Some(delta) = notice {
        if delta.new_revision != delta.base_revision + 1
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
        let earliest = Duration::from_secs((minutes - 1) * 60).saturating_sub(Duration::from_secs(5));
        let latest = Duration::from_secs(minutes * 60 + 30);
        let close = client.session_mut().service_liveness(latest).await;
        let elapsed = noticed.elapsed();
        if !matches!(&close, Err(error)
            if error.to_string().starts_with("transport I/O failed: "))
        {
            return Err(format!("step=reset-close observed={close:?}-after-{}s-for-{minutes}-minute-notice", elapsed.as_secs()).into());
        }
        if elapsed < earliest {
            return Err(format!("step=reset-close observed=closed-after-{}s-before-the-{minutes}-minute-window error={close:?}", elapsed.as_secs()).into());
        }
        evidence(&format!("W05a_CHECK step=reset-close after={}s reply={close:?}", elapsed.as_secs()));
        client.ended = true;
        client = row_connect_world(clients, 0xa6).await?;
        if client.join_snapshot().game_session_id == admitted_session
            || client.world_spatial().actor_position != saved
        {
            return Err(format!("step=post-reset-admission observed=session-{:?}-position-{:?} expected-new-session-position-{saved:?}", client.join_snapshot().game_session_id, client.world_spatial().actor_position).into());
        }
        evidence("W05a_CHECK step=post-reset-admission durable-position=retained");
        let cleanup = client.row_logout().await?;
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
    client: &mut RowClient,
    clients: &SeamClients<'_>,
    saved: ActorPosition,
) -> TestResult {
    let old = client.join_snapshot().game_session_id;
    let reply = client.row_logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout observed={reply:?}").into());
    }
    *client = row_connect_world(clients, 0xa6).await?;
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
    let cleanup = client.row_logout().await?;
    if cleanup.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=final-logout observed={cleanup:?}").into());
    }
    Ok(())
}
