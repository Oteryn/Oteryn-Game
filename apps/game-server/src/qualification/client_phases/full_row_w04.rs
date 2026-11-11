//! W04 player scenario, derived from the matrix row, the 2026-10-28 re-audit,
//! and the production raid scheduler/runner named there:
//!
//! 1. Admit an ordinary node-boot character and send RAID_SCHEDULE_QUERY; expect
//!    an accepted, well-formed empty or current durable schedule projection.
//! 2. Send a malformed RAID_SCHEDULE_QUERY payload; expect a rejected command,
//!    no payload, and then a successful query proving the refusal changed no state.
//! 3. Remain admitted and repeatedly send RAID_SCHEDULE_QUERY while the server's
//!    schedule runner evaluates the catalogue; expect a transition to the
//!    `raid:badger` running state and a raid warning/advance ServerMessage delta
//!    when the selected definition contains an announcement.
//! 4. Keep querying through the wave; expect committed_steps to advance and
//!    active_creatures to become nonzero, proving the server realized a spawn.
//! 5. Keep querying through completion; expect running=false with the raid key,
//!    committed steps, and persistent active creature count retained.
//! 6. Log out, obtain a fresh admission for the same character, query again, and
//!    expect exactly the completed durable projection observed before logout.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::raid_schedule::{
    CAPABILITY_RAID_SCHEDULE_V1, COMMAND_TYPE_RAID_SCHEDULE_QUERY, RaidScheduleState,
    decode_raid_schedule_state,
};
use oteryn_protocol_oteryn::zones_nologout::ServerMessage;

const ACTIONS: &[&str] = &[
    "initial-query",
    "malformed-refusal",
    "schedule-fired",
    "announcement",
    "wave-spawn",
    "completion",
    "fresh-admission",
];

struct Row {
    passed: std::collections::BTreeSet<&'static str>,
    invalid: bool,
    started: std::time::Instant,
}

impl Row {
    fn new() -> Self {
        Self {
            passed: std::collections::BTreeSet::new(),
            invalid: false,
            started: std::time::Instant::now(),
        }
    }

    fn pass(&mut self, action: &'static str, detail: &str) {
        if !ACTIONS.contains(&action) || !self.passed.insert(action) {
            self.invalid = true;
            return;
        }
        evidence(&format!("W04_CHECK {action} PASS {detail}"));
    }

    fn finish(self, result: TestResult) {
        for action in ACTIONS {
            if !self.passed.contains(action) {
                evidence(&format!("W04_GAP step={action} observed=not-completed"));
            }
        }
        if let Err(error) = &result {
            evidence(&format!("W04_GAP observed={error}"));
        }
        let complete = !self.invalid
            && result.is_ok()
            && ACTIONS.iter().all(|action| self.passed.contains(action));
        evidence(&format!(
            "e2e_all scenario=W04.full status={} seconds={:.3}",
            if complete { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64()
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = row_complete(journey(&mut row, clients)).await;
    row.finish(match result {
        Ok(result) => result,
        Err(_) => Err(
            "step=schedule-fired observed=journey-deadline server-needed=eligible-raid-firing"
                .into(),
        ),
    });
}

async fn exchange(
    client: &mut DevClientSession,
    payload: &[u8],
) -> TestResult<oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse> {
    Ok(client
        .session_mut()
        .gated_command(
            CAPABILITY_RAID_SCHEDULE_V1,
            COMMAND_TYPE_RAID_SCHEDULE_QUERY,
            payload,
        )
        .await?)
}

async fn query(client: &mut DevClientSession) -> TestResult<RaidScheduleState> {
    let reply = exchange(client, &[]).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(
            format!("raid query observed={reply:?} server-needed=accepted-schedule-query").into(),
        );
    }
    decode_raid_schedule_state(&reply.payload)
        .map_err(|error| format!("raid query observed=malformed-state:{error:?}").into())
}

fn take_raid_notice(client: &mut DevClientSession) -> Option<ServerMessage> {
    client
        .take_events()
        .into_iter()
        .find_map(|event| match event {
            oteryn_dev_client::SessionEvent::ServerMessage(delta) => match delta.value {
                message @ (ServerMessage::Warning(_) | ServerMessage::Advance(_)) => Some(message),
                ServerMessage::StatusText(_) => None,
            },
            _ => None,
        })
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(190);
    let mut client = connect_world(clients, 0xb4).await?;
    let mut row_needs_logout_0 = true;
    let row_result: TestResult<_> = async {
    if !client
        .selected_capabilities()
        .contains(&CAPABILITY_RAID_SCHEDULE_V1)
    {
        return Err("step=initial-query observed=capability-149-not-selected server-needed=bundle-world-raid-capability".into());
    }

    let initial = query(&mut client).await?;
    row.pass("initial-query", &format!("state={initial:?}"));

    let refused = exchange(&mut client, &[0]).await?;
    if refused.status != CommandStatus::Rejected || !refused.payload.is_empty() {
        return Err(format!(
            "step=malformed-refusal observed={refused:?} server-needed=empty-rejection"
        )
        .into());
    }
    let after_refusal = query(&mut client).await?;
    if after_refusal != initial {
        return Err(format!(
            "step=malformed-refusal observed=state-changed:{initial:?}->{after_refusal:?}"
        )
        .into());
    }
    row.pass("malformed-refusal", "status=Rejected state=unchanged");

    client.take_events();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(175);
    let mut noticed = false;
    let mut running = None;
    let mut spawned = None;
    let completed = loop {
        if let Some(notice) = take_raid_notice(&mut client) {
            noticed = true;
            if !row.passed.contains("announcement") {
                row.pass("announcement", &format!("delta={notice:?}"));
            }
        }
        let state = query(&mut client).await?;
        if state.running && running.is_none() {
            if state.raid_key != "raid:badger" {
                return Err(
                    format!("step=schedule-fired observed=unexpected-key:{state:?}").into(),
                );
            }
            row.pass("schedule-fired", &format!("state={state:?}"));
            running = Some(state.clone());
        }
        if let Some(started) = &running
            && state.running
            && state.committed_steps > started.committed_steps
            && state.active_creatures > 0
            && spawned.is_none()
        {
            row.pass(
                "wave-spawn",
                &format!("started={started:?} advanced={state:?}"),
            );
            spawned = Some(state.clone());
        }
        if running.is_some() && !state.running {
            let Some(spawned) = &spawned else {
                return Err(format!(
                    "step=wave-spawn observed=raid-completed-without-observed-step-advance:{state:?} server-needed=committed-steps-advance-and-active-creatures"
                )
                .into());
            };
            if state.raid_key != "raid:badger"
                || state.committed_steps < spawned.committed_steps
                || state.active_creatures < spawned.active_creatures
            {
                return Err(format!(
                    "step=completion observed={state:?} wave-spawn={spawned:?} server-needed=stopped-badger-with-retained-progress-and-creatures"
                )
                .into());
            }
            row.pass(
                "completion",
                &format!("wave-spawn={spawned:?} completed={state:?}"),
            );
            break state;
        }
        if tokio::time::Instant::now() >= deadline {
            let step = if running.is_none() {
                "schedule-fired"
            } else if spawned.is_none() {
                "wave-spawn"
            } else {
                "completion"
            };
            return Err(format!("step={step} observed=timeout:last-state:{state:?}:notice={noticed} server-needed=due-raid-lifecycle").into());
        }
        client.service_liveness(Duration::from_secs(1)).await?;
    };

    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=fresh-admission observed=logout-refused".into());
    }
    row_needs_logout_0 = false;
    if row_needs_logout_0 { row_logout(&mut client).await?; row_needs_logout_0 = false; }
    let mut fresh = connect_world(clients, 0xb5).await?;
    let row_needs_logout_1 = true;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    let restored = query(&mut fresh).await?;
    if restored != completed {
        return Err(format!(
            "step=fresh-admission observed=state-mismatch:{completed:?}->{restored:?}"
        )
        .into());
    }
    row.pass("fresh-admission", &format!("state={restored:?}"));
    if !noticed {
        return Err("step=announcement observed=no-warning-or-advance-delta server-needed=scheduled-raid-with-announcement".into());
    }
    Ok(())

    }).await.unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_1 { row_logout(&mut fresh).await?; }
    row_result

    }.await;
    if row_needs_logout_0 {
        row_logout(&mut client).await?;
    }
    row_result
}
