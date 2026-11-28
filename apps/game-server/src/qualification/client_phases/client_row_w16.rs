//! W16 client replay capture: the player walks to the Summer 2026 NPC Wayland Smythers
//! (`TARGET_WALK`) and says `hi`, `danger`, `name`, `bye`. The driver asserts each server
//! disposition and catalogue reply, then replays the original server envelopes.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, EntityRef, SessionEvent};
use oteryn_protocol_oteryn::npc_service as wire;

const NPC: &str = "oteryn:npc.wayland_smythers";
const NAME: &str = "Wayland Smythers";
// TibiaWiki's Venore placement, independent of the compiled bundle.
const VENORE: ActorPosition = ActorPosition { x: 32960, y: 32075, floor: 6 };
const ENTRY: ActorPosition = ActorPosition { x: 32369, y: 32238, floor: 7 };
const APPROACH: ActorPosition = ActorPosition { x: 32959, y: 32076, floor: 6 };
const LANDING: ActorPosition = ActorPosition { x: 32959, y: 32075, floor: 6 };

const STEPS: [(&str, &str); 4] = [
    ("hi", "Ah, a visitor. Allow me to introduce myself. Wayland Smythers, in service of science. I am here in search of esteemed heroes, such as yourself, to avert a grave {danger}."),
    ("danger", "I am but a humble messenger, sent by a scientist in desperate need. Please, {hear} me out. I have no intention to deceive you."),
    ("name", "Wayland Smythers, in service of science."),
    ("bye", "Farewell, and do be careful."),
];

fn validate_reply(npc: EntityRef, step: (&str, &str), conversation: &wire::NpcConversation) -> TestResult {
    let (text, line) = step;
    if conversation.lines.as_slice() != [line] {
        return Err(format!("step={text} expected={line:?} observed={:?}", conversation.lines).into());
    }
    let expected = (text != "bye").then_some((npc, wire::NpcConversationState::Open));
    if conversation.open != expected {
        return Err(format!("step={text} expected={expected:?} observed={:?}", conversation.open).into());
    }
    Ok(())
}

fn expect_position(step: &str, actual: ActorPosition, expected: ActorPosition) -> TestResult {
    if actual != expected {
        return Err(format!("step={step} expected={expected:?} observed={actual:?}").into());
    }
    Ok(())
}

fn is_wayland(kind: EntityKind, name: Option<&str>, position: ActorPosition) -> bool {
    kind == EntityKind::Npc && name == Some(NAME) && position == VENORE
}

fn approach_route(
    from: ActorPosition,
    goal: TilePos,
    resolve: impl Fn(TilePos, StepDirection) -> Option<TilePos>,
) -> TestResult<Vec<StepDirection>> {
    expect_position("normal-admission", from, ENTRY)?;
    let path = find_route(tile(from)?, |at| at == goal, resolve)
        .map_err(|visited| format!("W16 no compiled walking route from {from:?} to {goal:?}; visited={visited}"))?;
    if path.is_empty() {
        return Err("W16 empty approach route".into());
    }
    Ok(path)
}

async fn walk_from_admission(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: Vec<StepDirection>,
) -> TestResult {
    let landing = |from, direction| destination_with_teleports(world, from, direction, false);
    evidence(&format!("W16_ROUTE from={ENTRY:?} approach={APPROACH:?} steps={}", path.len()));
    for direction in path {
        let before = tile(client.world_spatial().actor_position)?;
        let expected = landing(before, direction).ok_or("W16 compiled step has no landing")?;
        let outcome = client.step(direction).await?;
        if outcome.disposition != StepDisposition::Moved || outcome.world_spatial_delta.is_none()
            || tile(client.world_spatial().actor_position)? != expected
        {
            return Err(format!("W16 route step from={before:?} direction={direction:?} expected={expected:?} observed={outcome:?}").into());
        }
        if outcome.step_wait_ms > 0 {
            client.service_liveness(Duration::from_millis(u64::from(outcome.step_wait_ms))).await?;
        }
    }
    expect_position("walk-approach", client.world_spatial().actor_position, APPROACH)?;
    let reply = client.session_mut().target_walk(LANDING).await?;
    super::full_row_w16::require_target_walk(NPC, reply.status, &reply.payload)?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    while client.world_spatial().actor_position == APPROACH {
        if tokio::time::Instant::now() >= deadline {
            return Err("W16 final Target Walk did not leave the approach tile".into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
    expect_position("target-walk-landing", client.world_spatial().actor_position, LANDING)
}

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("W16_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

struct CapturedDialogue {
    record: String,
    frame: Vec<u8>,
    sequence: u64,
    base: u64,
    revision: u64,
    conversation: wire::NpcConversation,
}

async fn talk(
    client: &mut DevClientSession,
    npc: EntityRef,
    step: (&str, &str),
) -> TestResult<CapturedDialogue> {
    let (text, _) = step;
    tokio::time::sleep(Duration::from_millis(260)).await;
    let payload = wire::encode_npc_talk_intent(&wire::NpcTalkIntent {
        npc_actor: npc,
        text: text.into(),
    })
    .map_err(|error| format!("step={text} encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_NPC_SERVICE_V1,
            wire::COMMAND_TYPE_NPC_TALK_INTENT,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step={text} command status {:?}", reply.status).into());
    }
    let disposition = wire::decode_npc_intent_result(&reply.payload)
        .map_err(|error| format!("step={text} result: {error:?}"))?;
    if disposition != wire::NpcIntentDisposition::Ok {
        return Err(format!("step={text} observed {disposition:?}").into());
    }
    let delta = loop {
        let delta = client
            .session_mut()
            .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1)
            .await?;
        if delta.server_sequence > reply.server_sequence {
            break delta;
        }
    };
    let conversation = wire::decode_npc_conversation(&delta.payload)
        .map_err(|error| format!("step={text} conversation: {error:?}"))?;
    validate_reply(npc, step, &conversation)?;
    capture(&format!("w16-{text}-conversation.bin"), &delta.payload);
    let open = conversation.open.map_or_else(
        || "-".to_owned(),
        |(actor, _)| {
            let identity: String = actor.identity.iter().map(|byte| format!("{byte:02x}")).collect();
            format!("{identity}:{}", actor.generation)
        },
    );
    let record = format!(
        "{text}\t{}\t{}\t{}\t{}\t{open}\t{}\n",
        wire::STATE_DOMAIN_NPC_CONVERSATION,
        delta.server_sequence,
        delta.base_revision,
        delta.new_revision,
        conversation.lines.join("\u{1f}")
    );
    Ok(CapturedDialogue {
        record,
        frame: delta.frame,
        sequence: delta.server_sequence,
        base: delta.base_revision,
        revision: delta.new_revision,
        conversation,
    })
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let npc = metadata
        .npcs
        .npcs
        .iter()
        .find(|npc| npc.key == NPC)
        .ok_or("W16 client capture: Wayland Smythers absent from the bundle")?;
    let world = full_e2ef7::qualification_world(clients)?;
    let fixture = character_prep::dedicated_character(clients, "NODE_BOOT_W16_ACCOUNT").await?;
    let fixture_clients = fixture.clients(clients);
    let initial = &fixture_clients;
    let mut client = connect_character(initial, 0x89, 0).await?;
    let path = approach_route(client.world_spatial().actor_position, tile(APPROACH)?, |from, direction| {
        destination_with_teleports(world, from, direction, false)
    });
    if !npc.placements.iter().any(|placement| {
        i32::from(placement.x) == VENORE.x
            && i32::from(placement.y) == VENORE.y
            && -i16::from(placement.floor) == VENORE.floor
    }) {
        if let Err(error) = &path {
            evidence(&format!("W16_ROUTE_BLOCKER {error}"));
        }
        return Err(format!("W16 Global Wayland placement absent: expected={VENORE:?} compiled={:?}", npc.placements).into());
    }
    walk_from_admission(&mut client, world, path?).await?;
    let landing = client.world_spatial().actor_position;
    let actor = client
        .world_entities()
        .and_then(|entities| {
            entities
                .iter()
                .find(|entry| is_wayland(
                    entry.kind,
                    entry.display_name.as_ref().and_then(|name| name.as_str().ok()),
                    entry.position,
                ))
        })
        .map(|entry| entry.entity)
        .ok_or("W16 client capture: Wayland Smythers not visible")?;
    let session = client.join_snapshot().game_session_id;
    let capture_dir = Path::new(&required("NODE_BOOT_WORLD_BUNDLE")?)
        .with_file_name(format!("w16-replay-{}", uuid_text(session.as_bytes())));
    std::fs::create_dir(&capture_dir)?;
    let baseline_revision = client.session_mut()
        .gated_snapshot(wire::STATE_DOMAIN_NPC_CONVERSATION)
        .ok_or("W16 conversation snapshot absent")?.revision;
    let baseline_sequence = client.last_server_sequence();
    let mut captured = Vec::new();
    let mut failure = None;
    for step in STEPS {
        match talk(&mut client, actor, step).await {
            Ok(dialogue) => {
                expect_position(step.0, client.world_spatial().actor_position, landing)?;
                std::fs::write(capture_dir.join(step.0), &dialogue.frame)?;
                captured.push(dialogue);
            }
            Err(error) => {
                failure = Some(error);
                break;
            }
        }
    }
    expect_position("before-logout", client.world_spatial().actor_position, landing)?;
    super::full_row_w16::logout_w16(&mut client).await?;
    if let Some(error) = failure {
        return Err(error);
    }
    let mut fresh = connect_character(initial, 0x8a, 0).await?;
    if fresh.join_snapshot().game_session_id == session {
        return Err("W16 relog reused the original session".into());
    }
    expect_position("relog-walk-landing", fresh.world_spatial().actor_position, landing)?;
    super::full_row_w16::logout_w16(&mut fresh).await?;
    let frames = STEPS.iter()
        .map(|(step, _)| std::fs::read(capture_dir.join(step)))
        .collect::<Result<Vec<_>, _>>()?;
    let replayed = client.session_mut().replay_retired_npc_conversation(
        baseline_revision, baseline_sequence, &frames,
    )?;
    if replayed.len() != STEPS.len() || captured.len() != STEPS.len() {
        return Err("W16 incomplete conversation replay".into());
    }
    for (event, expected) in replayed.iter().zip(&captured) {
        let SessionEvent::NpcConversation(delta) = event else {
            return Err("W16 replay did not emit a conversation".into());
        };
        if (delta.server_sequence, delta.base_revision, delta.new_revision)
            != (expected.sequence, expected.base, expected.revision)
            || delta.value != expected.conversation
        {
            return Err("W16 replay conversation differs from the live frame".into());
        }
    }
    let expected: String = captured.iter().map(|frame| frame.record.as_str()).collect();
    capture("w16-expected.txt", expected.as_bytes());
    Ok(format!("original-envelope-replay session={} frames={}", uuid_text(session.as_bytes()), replayed.len()))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W16.client status=PASS evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W16.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=W16.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approach_rejects_adjacent_fixture_and_requires_connected_ground() -> TestResult {
        assert!(approach_route(APPROACH, tile(APPROACH)?, |_, _| None).is_err());
        assert!(approach_route(ENTRY, tile(APPROACH)?, |_, _| None).is_err());
        let from = tile(ENTRY)?;
        let next = TilePos { x: from.x + 1, ..from };
        assert_eq!(approach_route(ENTRY, next, |at, direction| {
            (at == from && direction == StepDirection::East).then_some(next)
        })?, [StepDirection::East]);
        Ok(())
    }

    #[test]
    fn wayland_requires_global_tile_npc_kind_and_exact_name() {
        assert!(is_wayland(EntityKind::Npc, Some(NAME), VENORE));
        assert!(!is_wayland(EntityKind::Npc, Some("Another NPC"), VENORE));
        assert!(!is_wayland(EntityKind::Creature, Some(NAME), VENORE));
        assert!(!is_wayland(EntityKind::Npc, Some(NAME), ActorPosition { x: 32961, y: 32076, ..VENORE }));
    }

    #[test]
    fn dialogue_rejects_greeting_as_name_extra_lines_and_confirmation() {
        let npc = EntityRef { identity: [1; 16], generation: 1 };
        let mut dialogue = wire::NpcConversation {
            open: Some((npc, wire::NpcConversationState::Open)),
            lines: vec![STEPS[2].1.into()],
        };
        assert!(validate_reply(npc, STEPS[2], &dialogue).is_ok());
        dialogue.lines.push("extra".into());
        assert!(validate_reply(npc, STEPS[2], &dialogue).is_err());
        dialogue.lines = vec![STEPS[0].1.into()];
        assert!(validate_reply(npc, STEPS[2], &dialogue).is_err());
        dialogue.open = Some((npc, wire::NpcConversationState::Confirming));
        assert!(validate_reply(npc, STEPS[0], &dialogue).is_err());
        dialogue.lines = vec![STEPS[3].1.into()];
        assert!(validate_reply(npc, STEPS[3], &dialogue).is_err());
        dialogue.open = None;
        assert!(validate_reply(npc, STEPS[3], &dialogue).is_ok());
    }

    #[test]
    fn a_displaced_position_cannot_become_the_relog_baseline() {
        assert!(expect_position("after-name", LANDING, LANDING).is_ok());
        assert!(expect_position("after-name", APPROACH, LANDING).is_err());
        assert!(expect_position("relog", APPROACH, LANDING).is_err());
    }
}
