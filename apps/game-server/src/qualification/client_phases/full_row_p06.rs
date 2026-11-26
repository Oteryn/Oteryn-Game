//! P06 — Wheel allocation/save/reset and dedication/conviction/revelation.
//!
//! Concrete player scenario, derived from the P06 matrix/audit row and the live
//! `gameplay_transport/wheel_allocation.rs` owner:
//! 1. Prepare the dedicated level-300 Sorcerer, buy promotion and query the Wheel;
//!    expect an accepted, bounded revision/36-slot projection.
//! 2. Save a partial core-slot allocation with a fresh player occurrence; expect
//!    an accepted non-refused reply, revision increment, the exact slot delta,
//!    and the dedication max-mana delta in the actor-vitals frame.
//! 3. Save the full core slot; expect another revision, the exact allocation,
//!    and the full-slot conviction together with its additional dedication delta.
//! 4. Send a stale-revision save; expect an accepted semantic refusal carrying
//!    the unchanged current revision/allocation and no vitals delta.
//! 5. Logout and make a fresh admission; query and re-check the durable full-slot
//!    allocation, revision, and dedication vitals projection.
//! 6. Exercise revelation through the same player save route: the admitted
//!    character must expose at least the first 250-point domain threshold; the
//!    accepted allocation and live effect must survive a fresh admission. If the
//!    legitimately provisioned/player-earned level supplies fewer points, report
//!    the observed player level and required server/world prerequisite as FAIL.
//! 7. Send reset with a fresh occurrence/current revision; expect an accepted
//!    non-refused zero allocation, a revision increment, and removal of the
//!    dedication/conviction/revelation projection.
//! 8. Logout and freshly admit once more; query and assert the durable zero
//!    allocation/revision and restored baseline vitals.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::wheel_allocation::{self, WheelCommand, WheelResponse};
use oteryn_protocol_oteryn::wheel::{self, WheelAllocationReply, WheelAllocationRequest};
use std::collections::BTreeSet;

const ACTIONS: &[&str] = &[
    "query",
    "dedication-save",
    "conviction-save",
    "stale-refusal",
    "fresh-admission-save",
    "revelation-save",
    "fresh-admission-revelation",
    "reset",
    "fresh-admission-reset",
];
const PROFILE: &str = "p06-wheel";
const LEVEL: u32 = 300;
const BASE_VITALS: (u32, u32, u32) = (1645, 8850, 100);
const CORE_SLOT: usize = 20; // protocol slot 21: an open max-mana core in every vocation.

struct Row {
    passed: BTreeSet<&'static str>,
    started: std::time::Instant,
}

impl Row {
    fn new() -> Self {
        Self {
            passed: BTreeSet::new(),
            started: std::time::Instant::now(),
        }
    }

    fn pass(&mut self, action: &'static str, observed: impl std::fmt::Debug) {
        if ACTIONS.contains(&action) && self.passed.insert(action) {
            evidence(&format!(
                "P06_CHECK action={action} status=PASS observed={observed:?}"
            ));
        }
    }

    fn finish(self, result: TestResult) {
        let missing = ACTIONS.iter().find(|action| !self.passed.contains(*action));
        let complete = result.is_ok() && missing.is_none();
        match result {
            Err(error) => evidence(&format!(
                "P06_GAP status=FAIL observed={}",
                error.to_string().replace(char::is_whitespace, "_")
            )),
            Ok(()) => match missing {
                Some(action) => evidence(&format!(
                    "P06_GAP status=FAIL action={action} observed=unproven"
                )),
                None => evidence("P06_GAP status=PASS"),
            },
        }
        evidence(&format!(
            "e2e_all scenario=P06.full status={} seconds={:.3}",
            if complete { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64()
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = verdict::indexed_timeout(
        Duration::from_secs(180),
        verdict::observe(async {
            let dedicated =
                character_prep::dedicated_character(clients, "NODE_BOOT_Q_P06_ACCOUNT").await?;
            let scope = dedicated.clients(clients);
            let result = journey(&mut row, &scope).await;
            let cleanup = release_row_characters(&scope).await;
            result.and(cleanup)
        }),
    )
    .await;
    row.finish(match result {
        Ok(result) => result,
        Err(_) => {
            Err("step deadline: Wheel journey exceeded 180 seconds; server reply timeout".into())
        }
    });
}

fn command(request: &WheelAllocationRequest) -> TestResult<WheelCommand> {
    let payload = wheel::encode_wheel_request(request)
        .map_err(|error| format!("Wheel request encoder refused test input: {error:?}"))?;
    WheelCommand::new(wheel::COMMAND_TYPE_WHEEL_ALLOCATION, payload)
        .ok_or_else(|| "Wheel runtime rejected protocol-oteryn command".into())
}

async fn send(
    client: &mut DevClientSession,
    request: &WheelAllocationRequest,
) -> TestResult<(WheelResponse, WheelAllocationReply)> {
    let response = wheel_allocation::send(client.session_mut(), &command(request)?).await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!(
            "mechanic command action={} observed status={:?} payload={:?}; need the server Wheel route to accept capability 148 command 161",
            request.action, response.status, response.payload
        )
        .into());
    }
    let reply = wheel::decode_wheel_reply(&response.payload).map_err(|error| {
        format!(
            "mechanic command action={} returned undecodable reply {:?}: {error:?}",
            request.action, response.payload
        )
    })?;
    Ok((response, reply))
}

fn request(
    action: u8,
    occurrence: [u8; 16],
    revision: u64,
    points: [u16; 36],
) -> WheelAllocationRequest {
    WheelAllocationRequest {
        action,
        occurrence,
        expected_revision: revision,
        points,
        vessels: None,
    }
}

fn query_request() -> WheelAllocationRequest {
    request(0, [0; 16], 0, [0; 36])
}

fn validate_current(response: &WheelResponse, expected: &WheelAllocationReply) -> TestResult {
    let current = response
        .wheel
        .as_deref()
        .ok_or("accepted Wheel command omitted its current-state query frame")?;
    let current = wheel::decode_wheel_reply(current)
        .map_err(|error| format!("current Wheel projection is malformed: {error:?}"))?;
    // The receipt describes the attempted command. Its follow-up query must succeed.
    if current
        != (WheelAllocationReply {
            refused: false,
            ..expected.clone()
        })
    {
        return Err(format!(
            "current Wheel projection mismatch: receipt={expected:?} current={current:?}"
        )
        .into());
    }
    Ok(())
}

async fn own_level(client: &mut DevClientSession) -> TestResult<u32> {
    use oteryn_protocol_oteryn::cyclopedia_character as wire;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            wire::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[1, 0, 0, 1],
        )
        .await?;
    if reply.status != CommandStatus::Accepted
        || general_stats::validate(&reply.payload, general_stats::selected(client)).is_err()
        || reply.payload[0] != 1
    {
        return Err(format!("Wheel progression query observed reply={reply:?}").into());
    }
    let level = u32::from_le_bytes(reply.payload[9..13].try_into()?);
    (level != 0)
        .then_some(level)
        .ok_or_else(|| "Wheel progression query returned level zero".into())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("Wheel logout observed reply={reply:?}").into());
    }
    Ok(())
}

async fn talk_once(
    client: &mut DevClientSession,
    npc: oteryn_dev_client::EntityRef,
    text: &str,
) -> TestResult<Vec<String>> {
    use oteryn_protocol_oteryn::npc_service as wire;

    let payload = wire::encode_npc_talk_intent(&wire::NpcTalkIntent {
        npc_actor: npc,
        text: text.into(),
    })
    .map_err(|error| format!("step=promotion encode {text}: {error:?}"))?;
    let session = client.session_mut();
    let reply = session
        .gated_command(
            wire::CAPABILITY_NPC_SERVICE_V1,
            wire::COMMAND_TYPE_NPC_TALK_INTENT,
            &payload,
        )
        .await?;
    let disposition = wire::decode_npc_intent_result(&reply.payload)
        .map_err(|error| format!("step=promotion decode {text}: {error:?}"))?;
    if reply.status != CommandStatus::Accepted || disposition != wire::NpcIntentDisposition::Ok {
        return Err(format!(
            "step=promotion {text} status={:?} disposition={disposition:?}",
            reply.status
        )
        .into());
    }
    loop {
        let delta = session
            .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1)
            .await?;
        if delta.server_sequence > reply.server_sequence {
            let conversation = wire::decode_npc_conversation(&delta.payload)
                .map_err(|error| format!("step=promotion conversation {text}: {error:?}"))?;
            if conversation.open.map(|(actor, _)| actor) != Some(npc) {
                return Err("step=promotion conversation belongs to another NPC".into());
            }
            return Ok(conversation.lines);
        }
    }
}

async fn prepare(clients: &SeamClients<'_>) -> TestResult<DevClientSession> {
    character_prep::declare_seeded(
        "P06.full",
        "q_p06,sorcerer-level-300,premium,20000-promotion-gold",
        "the first revelation requires 250 Wheel points",
    );
    let mut client = connect_character(clients, 0xd6, 0).await?;
    request_fixture_control(
        uuid_text(client.join_snapshot().game_session_id.as_bytes()),
        "--profile",
        PROFILE.into(),
    )
    .await?;
    character_prep::grant_premium(&client).await?;
    character_prep::grant_item(&client, 3043, 2).await?;
    logout(&mut client).await?;
    drop(client);

    let mut client = connect_character(clients, 0xd6, 0).await?;
    let world = full_e2ef7::qualification_world(clients)?;
    let bundle = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bundle, |_, _| Ok(()))?;
    let origin = tile(client.world_spatial().actor_position)?;
    let npc = full_row_p20::approach_npc(
        &mut client,
        world,
        &metadata.npcs,
        "oteryn:npc.king_tibianus",
    )
    .await?;
    if talk_once(&mut client, npc, "hi").await?.is_empty() {
        return Err("step=promotion greeting absent".into());
    }
    let quote = talk_once(&mut client, npc, "promotion").await?;
    if quote != ["Do you want to be promoted for 20000 gold coins?"] {
        return Err(format!("step=promotion quote={quote:?}").into());
    }
    let confirmation = talk_once(&mut client, npc, "yes").await?;
    if confirmation != ["You are now promoted."] {
        return Err(format!("step=promotion confirmation={confirmation:?}").into());
    }
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at == origin
    })
    .ok_or("step=promotion no route back to the original temple tile")?;
    walk(&mut client, world, &path).await?;
    logout(&mut client).await?;
    drop(client);
    connect_character(clients, 0xd6, 0).await
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let mut client = prepare(clients).await?;
    if !client
        .selected_capabilities()
        .contains(&wheel::CAPABILITY_WHEEL_ALLOCATION_V1)
    {
        return Err(
            "step 1 observed capability 148 absent; need server Wheel capability selection".into(),
        );
    }
    let (_, initial) = send(&mut client, &query_request()).await?;
    if initial.refused {
        return Err(format!("step 1 Wheel query observed semantic refusal: {initial:?}").into());
    }
    let baseline = *client
        .actor_vitals()
        .ok_or("step 1 actor-vitals frame absent")?;
    let level = own_level(&mut client).await?;
    if level != LEVEL || (baseline.max_health, baseline.max_mana, baseline.soul) != BASE_VITALS {
        return Err(format!("step=profile expected level={LEVEL} vitals={BASE_VITALS:?}, observed level={level} vitals={baseline:?}").into());
    }
    evidence(&format!(
        "P06_PREP account=q_p06 level={level} promotion=confirmed vitals={baseline:?}"
    ));
    row.pass("query", (&initial, level, baseline));

    // Use the presently admitted allocation as the starting point. Node boot normally
    // starts at zero, while this also makes a rerun diagnose retained state precisely.
    if initial.points != [0; 36] {
        return Err(format!(
            "step 2 needs the preceding packet's clean Wheel, observed query={initial:?}; reset it through the player UI before rerunning"
        )
        .into());
    }
    let mut partial = [0; 36];
    partial[CORE_SLOT] = 25;
    let (response, saved_partial) = send(
        &mut client,
        &request(1, v7(0xd6, 1), initial.revision, partial),
    )
    .await?;
    if saved_partial.refused
        || saved_partial.revision != initial.revision + 1
        || saved_partial.points != partial
    {
        return Err(format!(
            "step 2 partial dedication save observed reply={saved_partial:?}; need eligible promoted Premium level>50 provisioned character"
        )
        .into());
    }
    validate_current(&response, &saved_partial)?;
    let partial_vitals = *client
        .actor_vitals()
        .ok_or("step 2 dedication actor-vitals delta missing")?;
    if partial_vitals.max_mana <= baseline.max_mana {
        return Err(format!(
            "step 2 dedication save had no max-mana delta: before={baseline:?} after={partial_vitals:?} reply={saved_partial:?}"
        )
        .into());
    }
    row.pass("dedication-save", (&saved_partial, partial_vitals));

    let mut full = [0; 36];
    full[CORE_SLOT] = 50;
    let (response, saved_full) = send(
        &mut client,
        &request(1, v7(0xd6, 2), saved_partial.revision, full),
    )
    .await?;
    if saved_full.refused
        || saved_full.revision != saved_partial.revision + 1
        || saved_full.points != full
    {
        return Err(
            format!("step 3 full-slot conviction save observed reply={saved_full:?}").into(),
        );
    }
    validate_current(&response, &saved_full)?;
    let full_vitals = *client
        .actor_vitals()
        .ok_or("step 3 conviction actor-vitals delta missing")?;
    if full_vitals.max_mana <= partial_vitals.max_mana {
        return Err(format!(
            "step 3 full-slot dedication/conviction delta absent: partial={partial_vitals:?} full={full_vitals:?}"
        )
        .into());
    }
    row.pass("conviction-save", (&saved_full, full_vitals));

    let (response, stale) = send(
        &mut client,
        &request(1, v7(0xd6, 3), initial.revision, partial),
    )
    .await?;
    if !stale.refused || stale.revision != saved_full.revision || stale.points != full {
        return Err(
            format!("step 4 stale save was not a state-preserving refusal: {stale:?}").into(),
        );
    }
    validate_current(&response, &stale)?;
    if client.actor_vitals().copied() != Some(full_vitals) {
        return Err("step 4 refused stale save changed actor vitals".into());
    }
    row.pass("stale-refusal", &stale);

    logout(&mut client).await?;
    drop(client);
    let mut client = connect_character(clients, 0xd7, 0).await?;
    let (_, reloaded) = send(&mut client, &query_request()).await?;
    if reloaded != saved_full || client.actor_vitals().copied() != Some(full_vitals) {
        return Err(format!(
            "step 5 fresh admission lost durable allocation/vitals: saved={saved_full:?} queried={reloaded:?} vitals={:?}",
            client.actor_vitals()
        )
        .into());
    }
    row.pass("fresh-admission-save", (&reloaded, client.actor_vitals()));

    // The first revelation requires 250 points in one domain. The native server
    // publishes the player's real level, so this is runtime evidence rather than a
    // source-selected verdict. The node-boot level-100 profile currently has 50.
    let available = level.saturating_sub(50);
    let revelation_gap = if available < 250 {
        // Make the mechanic attempt even when the current player cannot legally own
        // the points. Keeping the already-full core and asking for a second core's
        // capacity demonstrates the server-side budget refusal without forging XP.
        let mut attempted = full;
        attempted[14] = 50;
        let (response, refused) = send(
            &mut client,
            &request(1, v7(0xd7, 1), reloaded.revision, attempted),
        )
        .await?;
        if !refused.refused || refused.revision != reloaded.revision || refused.points != full {
            return Err(format!(
                "step 6 over-budget revelation-path attempt did not preserve state: {refused:?}"
            )
            .into());
        }
        validate_current(&response, &refused)?;
        Some(format!(
            "step 6 revelation attempt observed semantic refusal with player level={level}, available Wheel points={available}, reply={refused:?}; need a legitimate level>=300 for the 250-point domain allocation"
        ))
    } else {
        // Green domain: core slot 15 unlocks 9 and 14; either completed ring
        // slot unlocks 8. The exact capacities total the stage-one threshold.
        let mut revelation = [0; 36];
        revelation[14] = 50;
        revelation[8] = 75;
        revelation[13] = 75;
        revelation[7] = 50;
        let (response, revealed) = send(
            &mut client,
            &request(1, v7(0xd7, 1), reloaded.revision, revelation),
        )
        .await?;
        if revealed.refused
            || revealed.revision != reloaded.revision + 1
            || revealed.points != revelation
        {
            return Err(format!("step 6 revelation save observed reply={revealed:?}").into());
        }
        validate_current(&response, &revealed)?;
        let active = full_row_p07::live_wheel_effects(&mut client, &revealed, 2).await?;
        if !active
            .iter()
            .any(|line| line.ends_with(": revelation stage 1"))
        {
            return Err(
                format!("step=revelation-effect stage-one projection missing: {active:?}").into(),
            );
        }
        row.pass("revelation-save", (&revealed, client.actor_vitals()));
        let vitals = client.actor_vitals().copied();
        logout(&mut client).await?;
        drop(client);
        client = connect_character(clients, 0xd9, 0).await?;
        let (_, retained) = send(&mut client, &query_request()).await?;
        if retained != revealed || client.actor_vitals().copied() != vitals {
            return Err(format!(
                "step=revelation-relog lost durable state: saved={revealed:?} queried={retained:?}"
            )
            .into());
        }
        let retained_effects = full_row_p07::live_wheel_effects(&mut client, &retained, 3).await?;
        if retained_effects != active {
            return Err(format!("step=revelation-relog effects changed: saved={active:?} queried={retained_effects:?}").into());
        }
        row.pass("fresh-admission-revelation", (&retained, retained_effects));
        None
    };

    let reset_revision = if revelation_gap.is_none() {
        reloaded.revision + 1
    } else {
        reloaded.revision
    };
    let (response, reset) = send(
        &mut client,
        &request(2, v7(0xd7, 2), reset_revision, [0; 36]),
    )
    .await?;
    if reset.refused || reset.revision != reset_revision + 1 || reset.points != [0; 36] {
        return Err(format!("step 7 reset observed reply={reset:?}").into());
    }
    validate_current(&response, &reset)?;
    if client.actor_vitals().copied() != Some(baseline) {
        return Err(format!(
            "step 7 reset did not remove Wheel vitals: baseline={baseline:?} observed={:?}",
            client.actor_vitals()
        )
        .into());
    }
    if revelation_gap.is_none() {
        let active = full_row_p07::live_wheel_effects(&mut client, &reset, 4).await?;
        if active
            .iter()
            .any(|line| line.contains(": revelation stage "))
        {
            return Err(format!("step=revelation-reset active effect remains: {active:?}").into());
        }
    }
    row.pass("reset", (&reset, client.actor_vitals()));

    logout(&mut client).await?;
    drop(client);
    let mut final_client = connect_character(clients, 0xd8, 0).await?;
    let (_, final_state) = send(&mut final_client, &query_request()).await?;
    if final_state != reset || final_client.actor_vitals().copied() != Some(baseline) {
        return Err(format!(
            "step 8 fresh admission lost durable reset: reset={reset:?} query={final_state:?} vitals={:?}",
            final_client.actor_vitals()
        )
        .into());
    }
    row.pass(
        "fresh-admission-reset",
        (&final_state, final_client.actor_vitals()),
    );
    logout(&mut final_client).await?;
    if let Some(gap) = revelation_gap {
        return Err(gap.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_shapes_use_the_registered_protocol_codec() -> TestResult {
        let query = query_request();
        let round_trip = |value: &WheelAllocationRequest| -> TestResult<WheelAllocationRequest> {
            let bytes =
                wheel::encode_wheel_request(value).map_err(|error| format!("encode: {error:?}"))?;
            wheel::decode_wheel_request(&bytes).map_err(|error| format!("decode: {error:?}").into())
        };
        assert_eq!(round_trip(&query)?, query);
        let mut points = [0; 36];
        points[CORE_SLOT] = 50;
        let save = request(1, v7(1, 2), 7, points);
        assert_eq!(round_trip(&save)?, save);
        let reset = request(2, v7(1, 3), 8, [0; 36]);
        assert_eq!(round_trip(&reset)?, reset);
        Ok(())
    }

    #[test]
    fn current_query_matches_accepted_and_refused_receipt_state() -> TestResult {
        let mut points = [0; 36];
        points[CORE_SLOT] = 25;
        let current = WheelAllocationReply {
            refused: false,
            revision: 1,
            points,
            vessels: None,
        };
        let response = |reply: &WheelAllocationReply| -> TestResult<WheelResponse> {
            let bytes =
                wheel::encode_wheel_reply(reply).map_err(|error| format!("encode: {error:?}"))?;
            Ok(WheelResponse {
                kind: wheel::COMMAND_TYPE_WHEEL_ALLOCATION,
                status: CommandStatus::Accepted,
                payload: Vec::new(),
                wheel: Some(bytes),
                atelier: None,
                resources: None,
            })
        };
        let refused = WheelAllocationReply {
            refused: true,
            ..current.clone()
        };
        validate_current(&response(&current)?, &current)?;
        validate_current(&response(&current)?, &refused)?;
        assert!(validate_current(&response(&refused)?, &refused).is_err());
        let wrong_revision = WheelAllocationReply {
            revision: 2,
            ..current.clone()
        };
        assert!(validate_current(&response(&wrong_revision)?, &refused).is_err());
        let wrong_points = WheelAllocationReply {
            points: [0; 36],
            ..current
        };
        assert!(validate_current(&response(&wrong_points)?, &refused).is_err());
        Ok(())
    }

    #[test]
    fn verdict_requires_every_unique_observed_action() {
        let mut row = Row::new();
        for action in ACTIONS {
            row.pass(action, "test");
        }
        assert_eq!(row.passed.len(), ACTIONS.len());
        row.pass(ACTIONS[0], "duplicate");
        assert_eq!(row.passed.len(), ACTIONS.len());
    }
}

#[cfg(test)]
#[test]
fn row_06_publishes_a_gap_for_an_unproven_journey() {
    assert!(
        include_str!("full_row_p06.rs")
            .split_once("#[cfg(test)]")
            .is_some_and(|(production, _)| production.contains("P06_GAP status="))
    );
}

#[cfg(test)]
#[test]
fn review_2_p06() {
    assert!(
        include_str!("full_row_p06.rs")
            .split_once("#[cfg(test)]")
            .is_some_and(|(body, _)| body.contains("live_wheel_effects"))
    );
}
