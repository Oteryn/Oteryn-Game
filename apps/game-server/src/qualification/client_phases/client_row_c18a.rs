//! Captures the real C18a combat-state and spell-result payloads consumed by
//! `crates/session/src/domain_dispatch.rs::Session::apply_domain_delta` and
//! `crates/client-runtime/src/features/local-spell-action-bar/protocol.rs::run_cast`.

use super::*;
use oteryn_dev_client::client_runtime::combat::{CombatCommand, CombatReport, run_combat};
use oteryn_dev_client::client_runtime::spell::SpellFeedback;
use oteryn_dev_client::{EntityDetail, SessionEvent};
use oteryn_protocol_oteryn::{
    actor_spell::{self, SpellCastDisposition},
    presentation, target_move,
};

const SCORCH_MANA: u32 = 8;
const RAT_MAX_HP: u64 = 20;
const RETREAT_STEPS: usize = 64;
const CAST_TILE: TilePos = TilePos {
    x: 32490,
    y: 32169,
    floor: -7,
};
const RAT_TILE: TilePos = TilePos {
    x: 32491,
    y: 32169,
    floor: -7,
};
const WORDS: &str = "exevo infir flam hur";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::observe(capture(clients)).await {
        Ok(()) => evidence(
            "SEAM_EVIDENCE e2e_capture scenario=C18a.client status=CAPTURED evidence=live-client-directional-cast",
        ),
        Err(error) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C18a.client status=FAIL evidence={error}"
        )),
    }
}

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    let wrapper = oteryn_dev_client::qualification_wrapper_identity()?;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let book = native_spell_book()?;
    let spell = client_spell_journey::spell_index(book, WORDS)?;
    let definition = book.indexed(spell).ok_or("C18a served spell absent")?;
    if definition.mana != crate::spell::ManaCost::Fixed(SCORCH_MANA) || definition.soul != 0 {
        return Err("C18a Scorch must cost exactly 8 mana and 0 soul".into());
    }
    let spawn_cells = metadata
        .spawns
        .sources
        .iter()
        .flat_map(|source| {
            source.points.iter().map(|point| TilePos {
                x: point.x,
                y: point.y,
                floor: source.floor,
            })
        })
        .collect();
    let mut client = connect_character(clients, 0xca, 1).await?;
    if tile(client.world_spatial().actor_position)? != TOWN {
        return Err("C18a expected the initial town tile".into());
    }
    walk_once(&mut client, world, &spawn_cells, CAST_TILE).await?;
    let here = client.world_spatial().actor_position;
    let rat = {
        let mut rats = client
            .world_entities()
            .ok_or("C18a capture has no entity projection")?
            .iter()
            .filter(|entry| {
                entry.kind == oteryn_dev_client::EntityKind::Creature
                    && tile(entry.position).ok() == Some(RAT_TILE)
                    && entry.display_name.is_some_and(|name| {
                        name.as_str()
                            .is_ok_and(|name| name.eq_ignore_ascii_case("rat"))
                    })
            });
        let rat = *rats.next().ok_or("C18a capture has no adjacent Rat")?;
        if rats.next().is_some() {
            return Err("C18a capture has multiple adjacent Rats".into());
        }
        rat
    };
    let target = rat.entity;
    let combat_before = *client
        .session_mut()
        .combat_state()
        .ok_or("C18a missing combat state")?;
    if combat_before.target.is_some() {
        return Err("C18a directional cast requires no active attack target".into());
    }
    let facing = StepDirection::East;
    let intent =
        target_move::encode_target_move_intent(target_move::TargetMoveIntent::Turn(facing))
            .map_err(|error| format!("C18a turn encoding failed: {error:?}"))?;
    let turn = client
        .session_mut()
        .gated_command(
            target_move::CAPABILITY_TARGET_MOVE_V1,
            target_move::COMMAND_TYPE_TARGET_MOVE_INTENT,
            &intent,
        )
        .await?;
    require_mechanic_acceptance(turn.status, "C18a turn toward the observed Rat")?;
    if turn.payload != [8, 1] {
        return Err("C18a turn toward the observed Rat was refused".into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    let (caster, health_before) = loop {
        let entities = client
            .world_entities()
            .ok_or("C18a has no post-turn entity projection")?;
        let own = entities
            .iter()
            .find(|entry| &entry.entity.identity == entities.own_identity())
            .ok_or("C18a has no own entity projection")?;
        let selected = entities
            .iter()
            .find(|entry| entry.entity == target)
            .ok_or("C18a selected Rat generation left visibility during the turn")?;
        if own.position != here || selected.position != rat.position {
            return Err("C18a caster or selected Rat moved during the turn".into());
        }
        if matches!(own.detail, EntityDetail::Actor { direction, .. } if direction == facing) {
            let EntityDetail::Actor { health_percent, .. } = selected.detail else {
                return Err("C18a selected Rat has no creature health".into());
            };
            if health_percent == 0 {
                return Err("C18a selected Rat has zero health before the cast".into());
            }
            break (own.entity, health_percent);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("C18a did not observe the caster facing toward the observed Rat".into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    };
    client.take_events();
    let pre_cast_sequence = client.last_server_sequence();
    let before = *client.actor_vitals().ok_or("C18a caster vitals absent")?;
    let mut feedback = SpellFeedback::default();
    let command_id = client.next_command_id();
    let (report, mut exchange) = oteryn_dev_client::capture_qualification_exchange(run_combat(
        client.session_mut(),
        &mut feedback,
        CombatCommand::Cast(spell),
    ))
    .await;
    let report = report?;
    let (result_server_sequence, status, cast_payload) =
        received_cast_payload(&exchange.inbound, command_id).await?;
    require_mechanic_acceptance(status, "C18a directional Scorch")?;
    if report != CombatReport::Cast(SpellCastDisposition::Cast)
        || feedback.text() != Some("Spell cast")
    {
        return Err(format!("C18a Scorch cast refused: {report:?}").into());
    }
    let (vitals_sequence, vitals_payload) = first_cast_vitals(&exchange.inbound).await?;
    if !(pre_cast_sequence < result_server_sequence && result_server_sequence < vitals_sequence) {
        return Err(format!(
            "C18a first stats frame is not the cast charge: before={pre_cast_sequence} reply={result_server_sequence} stats={vitals_sequence}"
        ).into());
    }
    let after = actor_spell::decode_actor_vitals(&vitals_payload)
        .map_err(|error| format!("C18a malformed cast vitals: {error:?}"))?;
    if before.mana.checked_sub(SCORCH_MANA) != Some(after.mana) || before.soul != after.soul {
        return Err("C18a Scorch did not apply its exact mana and soul costs".into());
    }
    let mut regeneration = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    let mut health = None;
    let mut damage = None;
    let (health_sequence, health_after, damage_sequence, damage_value) = loop {
        for event in client.take_events() {
            match event {
                SessionEvent::ActorVitals(delta) if delta.server_sequence > vitals_sequence => {
                    regeneration.push((delta.server_sequence, delta.value.mana, delta.value.soul));
                }

                SessionEvent::WorldSpatialEntities(delta)
                    if delta.server_sequence > result_server_sequence =>
                {
                    if delta.value.leave.contains(&target) {
                        return Err("C18a Rat left visibility after the cast".into());
                    }
                    for entry in delta
                        .value
                        .update
                        .iter()
                        .filter(|entry| entry.entity == target)
                    {
                        if let EntityDetail::Actor { health_percent, .. }
                        | EntityDetail::ActorWithIcons { health_percent, .. } = entry.detail
                            && health_percent != health_before
                        {
                            if health
                                .replace((delta.server_sequence, health_percent))
                                .is_some()
                            {
                                return Err("C18a observed multiple Rat health changes".into());
                            }
                        }
                    }
                }
                SessionEvent::Presentation(batch)
                    if batch.server_sequence > result_server_sequence =>
                {
                    let has_cast_cue =
                        oteryn_dev_client::qualification_scorch_cue(&batch.value, here);
                    for effect in batch.value {
                        if let presentation::PresentationEventKind::ValueText {
                            kind: presentation::PresentationValueKind::DamageDealt,
                            position,
                            target: victim,
                            attacker: Some(source),
                            primary,
                            secondary,
                            ..
                        } = effect.kind
                            && source == caster
                            && victim == target
                        {
                            if !has_cast_cue
                                || position != rat.position
                                || primary.color != presentation::PresentationColor::Red
                                || secondary.is_some()
                                || damage
                                    .replace((batch.server_sequence, primary.value))
                                    .is_some()
                            {
                                return Err(
                                    "C18a expected one red damage value for the caster and Rat"
                                        .into(),
                                );
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        if let (Some((sequence, remaining)), Some((damage_sequence, value))) = (health, damage) {
            let loss = u64::from(health_before).checked_sub(u64::from(remaining));
            if remaining == 0 || loss != value.checked_mul(100).map(|percent| percent / RAT_MAX_HP)
            {
                return Err(format!("C18a red damage {value} disagrees with Rat health {health_before} -> {remaining}").into());
            }
            break (sequence, remaining, damage_sequence, value);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "C18a missing correlated Scorch damage: command={} reply_sequence={} target={target:?} before={health_before} health={health:?} damage={damage:?}",
                command_id, result_server_sequence
            ).into());
        }
        let (pumped, read) = oteryn_dev_client::capture_qualification_exchange(
            client.service_liveness(Duration::from_millis(20)),
        )
        .await;
        pumped?;
        exchange.inbound.extend(read.inbound);
    };
    let combat_after = *client
        .session_mut()
        .combat_state()
        .ok_or("C18a missing post-cast combat state")?;
    if combat_after.target.is_some() || combat_after.modes != combat_before.modes {
        return Err("C18a directional cast changed the attack target or fight modes".into());
    }
    evidence(&format!(
        "SEAM_EVIDENCE c18a_health command={} reply_sequence={} delta_sequence={health_sequence} target={target:?} before={health_before} after={health_after} damage={damage_value}",
        command_id, result_server_sequence
    ));
    let session = client.join_snapshot().game_session_id;
    let capabilities = client.selected_capabilities().to_vec();
    let health_payload = client
        .session_mut()
        .raw_world_entity_deltas()
        .iter()
        .find(|delta| delta.server_sequence == health_sequence)
        .ok_or("C18a original health delta payload absent")?
        .payload
        .clone();
    let damage_payload = received_damage_payload(&exchange.inbound, damage_sequence).await?;

    // Use a fixed prefix of the return route. The caster stays outdoors until
    // the ordinary combat lock clears.
    let retreat = route_avoiding(
        world,
        CAST_TILE,
        |at| at == TOWN,
        |at| !spawn_cells.contains(&at),
    )
    .ok_or("C18a fixed retreat route unavailable")?;
    let retreat = retreat
        .get(..RETREAT_STEPS)
        .ok_or("C18a retreat route too short")?;
    let rest = retreat
        .iter()
        .try_fold(CAST_TILE, |from, &direction| {
            destination(world, from, direction).filter(|&at| !is_logout_rest_tile(world, at))
        })
        .ok_or("C18a fixed retreat crosses a protection tile")?;
    walk_path(&mut client, world, retreat, rest, Some(&mut regeneration)).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    loop {
        let state = client
            .session_mut()
            .combat_state()
            .ok_or("C18a missing outdoor combat state")?;
        if !state.in_fight {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "C18a combat lock did not clear at the fixed outdoor tile {rest:?}"
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(200)).await?;
        collect_vitals(&mut client, Some(&mut regeneration));
        verdict::Harness::global().progress("wait-for-combat-lock");
    }
    let (result, detail, _, _) = logout_once(&mut client).await?;
    if result != Some(LogoutResult::Accepted) {
        return Err(format!("C18a first logout refused: {detail}").into());
    }
    let logout_vitals = *client
        .actor_vitals()
        .ok_or("C18a missing final logout vitals")?;
    let logout_sequence = client.last_server_sequence();
    collect_vitals(&mut client, Some(&mut regeneration));
    if !oteryn_dev_client::qualification_mana_history(
        (vitals_sequence, after.mana, after.soul),
        &regeneration,
        after.max_mana,
        (logout_sequence, logout_vitals.mana, logout_vitals.soul),
    ) {
        return Err(format!(
            "C18a unexplained mana or soul transition after Scorch: {regeneration:?}"
        )
        .into());
    }

    drop(client);
    let mut fresh = connect_character(clients, 0xcb, 1).await?;
    if fresh.join_snapshot().game_session_id == session {
        return Err("C18a relog reused the cast session".into());
    }
    if tile(fresh.world_spatial().actor_position)? != rest {
        return Err("C18a relog did not retain the fixed outdoor tile".into());
    }
    let fresh_vitals = *fresh.actor_vitals().ok_or("C18a missing relogged vitals")?;
    if (fresh_vitals.mana, fresh_vitals.soul) != (logout_vitals.mana, logout_vitals.soul) {
        return Err(format!("C18a relog vitals differ after Scorch: expected mana={} soul={}, observed mana={} soul={}", logout_vitals.mana, logout_vitals.soul, fresh_vitals.mana, fresh_vitals.soul).into());
    }
    evidence(&format!(
        "SEAM_EVIDENCE c18a_mana before={} pre_cast_sequence={pre_cast_sequence} cast_sequence={result_server_sequence} charge={} vitals_sequence={vitals_sequence} logout={} logout_sequence={logout_sequence} relog={}",
        before.mana, after.mana, logout_vitals.mana, fresh_vitals.mana
    ));
    walk_once(&mut fresh, world, &spawn_cells, CAST_TILE).await?;
    let relogged = fresh
        .world_entities()
        .and_then(|entities| entities.get(&target))
        .ok_or("C18a relog did not observe the same Rat generation")?;
    if !matches!(relogged.detail, EntityDetail::Actor { health_percent, .. }
        | EntityDetail::ActorWithIcons { health_percent, .. } if health_percent == health_after)
    {
        return Err(format!(
            "C18a relog Rat health differs from {health_after}: {:?}",
            relogged.detail
        )
        .into());
    }

    let root = std::path::PathBuf::from(required("E2E_ALL_CAPTURE_DIR")?);
    let cast_intent = sent_cast_payload(&exchange.outbound, command_id).await?;
    std::fs::write(root.join("c18a-intent.bin"), cast_intent)?;
    use std::os::unix::fs::MetadataExt;
    let capture_dir = std::fs::metadata(&root)?;
    let session = uuid_text(session.as_bytes());
    for (name, sequence, payload) in [
        (
            "c18a-cast.json",
            result_server_sequence,
            cast_payload.as_slice(),
        ),
        (
            "c18a-vitals.json",
            vitals_sequence,
            vitals_payload.as_slice(),
        ),
        ("c18a-health.json", health_sequence, health_payload.as_ref()),
        (
            "c18a-damage.json",
            damage_sequence,
            damage_payload.as_slice(),
        ),
    ] {
        std::fs::write(
            root.join(name),
            serde_json::to_vec(&serde_json::json!({
                "command_id": command_id,
                "game_session_id": session,
                "payload": payload,
                "server_sequence": sequence,
                "status": status as u32,
            }))?,
        )?;
    }
    std::fs::write(
        root.join("c18a-proof.json"),
        serde_json::to_vec(&serde_json::json!({
            "game_session_id": session,
            "capture_device": capture_dir.dev(),
            "capture_inode": capture_dir.ino(),
            "spell": spell.get(),
            "damage": damage_value,
            "damage_sequence": damage_sequence,
            "caster_identity": caster.identity,
            "caster_generation": caster.generation,
            "mana_before": before.mana,
            "mana_after": after.mana,
            "pre_cast_sequence": pre_cast_sequence,
            "vitals_sequence": vitals_sequence,
            "logout_sequence": logout_sequence,
            "wrapper": wrapper,
            "regeneration": regeneration,
            "mana_logout": logout_vitals.mana,
            "soul_logout": logout_vitals.soul,
            "mana_relog": fresh_vitals.mana,
            "soul_before": before.soul,
            "soul_after": after.soul,
            "soul_relog": fresh_vitals.soul,
            "combat_before": oteryn_protocol_oteryn::attack::encode_actor_combat_state(&combat_before).map_err(|error| format!("{error:?}"))?,
            "combat_after": oteryn_protocol_oteryn::attack::encode_actor_combat_state(&combat_after).map_err(|error| format!("{error:?}"))?,
            "relog_session_id": uuid_text(fresh.join_snapshot().game_session_id.as_bytes()),
            "command_id": command_id,
            "cast_sequence": result_server_sequence,
            "health_sequence": health_sequence,
            "identity": target.identity,
            "generation": target.generation,
            "before": health_before,
            "after": health_after,
            "capabilities": capabilities,
        }))?,
    )?;
    let capture_name = root
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .ok_or("C18a capture directory lacks a run name")?;
    evidence(&format!(
        "SEAM_EVIDENCE c18a_capture_binding directory={capture_name} session={session}"
    ));
    Ok(())
}

async fn received_cast_payload(
    mut inbound: &[u8],
    command_id: u64,
) -> TestResult<(u64, CommandStatus, Vec<u8>)> {
    use oteryn_protocol_oteryn::{MessageType, decode_command_result, decode_wire_envelope};
    while !inbound.is_empty() {
        let frame = oteryn_dev_client::native_session::read_frame(&mut inbound).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::CommandResult {
            let received = decode_command_result(envelope.payload())?;
            if received.command_id != command_id {
                return Err("C18a original cast reply does not match the exchange".into());
            }
            return Ok((
                envelope.server_sequence(),
                received.status,
                received.payload.to_vec(),
            ));
        }
    }
    Err("C18a original cast reply absent from inbound capture".into())
}

async fn walk_once(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    spawn_cells: &std::collections::BTreeSet<TilePos>,
    goal: TilePos,
) -> TestResult {
    let start = tile(client.world_spatial().actor_position)?;
    // Plan once from the pinned map. Spawn cells are occupied by design.
    let path = route_avoiding(
        world,
        start,
        |at| at == goal,
        |at| !spawn_cells.contains(&at),
    )
    .ok_or_else(|| format!("C18a fixed route unavailable: {start:?} -> {goal:?}"))?;
    walk_path(client, world, &path, goal, None).await
}

async fn walk_path(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
    goal: TilePos,
    mut vitals: Option<&mut Vec<(u64, u32, u32)>>,
) -> TestResult {
    for &direction in path {
        let from = tile(client.world_spatial().actor_position)?;
        let expected =
            destination(world, from, direction).ok_or("C18a route lost its planned landing")?;
        if expected == CAST_TILE {
            wait_for_cast_tile(client, vitals.as_deref_mut()).await?;
        }
        let outcome = client.step(direction).await?;
        require_mechanic_acceptance(outcome.status, "C18a first step attempt")?;
        let actual = tile(client.world_spatial().actor_position)?;
        if outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.is_none()
            || actual != expected
        {
            return Err(format!("C18a first step failed: from={from:?} direction={direction:?} expected={expected:?} actual={actual:?} outcome={outcome:?}").into());
        }
        verdict::Harness::global().progress("walk");
        collect_vitals(client, vitals.as_deref_mut());
        if outcome.step_wait_ms != 0 {
            client
                .service_liveness(Duration::from_millis(u64::from(outcome.step_wait_ms)))
                .await?;
        }
    }
    collect_vitals(client, vitals);
    if tile(client.world_spatial().actor_position)? != goal {
        return Err("C18a fixed route did not reach its goal".into());
    }
    Ok(())
}

async fn sent_cast_payload(mut outbound: &[u8], command_id: u64) -> TestResult<Vec<u8>> {
    use oteryn_protocol_oteryn::{MessageType, decode_wire_envelope};
    let mut intent = None;
    while !outbound.is_empty() {
        let frame = oteryn_dev_client::native_session::read_frame(&mut outbound).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::ClientCommand {
            let command = envelope.client_command(envelope.connection_generation())?;
            if command.command_id != command_id
                || command.command_type != oteryn_protocol_oteryn::actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT
                || intent.replace(command.payload.to_vec()).is_some()
            {
                return Err("C18a captured an unexpected outbound command".into());
            }
        }
    }
    intent.ok_or_else(|| "C18a original outbound cast intent absent".into())
}

async fn received_damage_payload(mut inbound: &[u8], sequence: u64) -> TestResult<Vec<u8>> {
    use oteryn_protocol_oteryn::{MessageType, decode_state_delta, decode_wire_envelope};
    while !inbound.is_empty() {
        let frame = oteryn_dev_client::native_session::read_frame(&mut inbound).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::StateDelta
            && envelope.server_sequence() == sequence
        {
            let delta = decode_state_delta(envelope.payload())?;
            if (delta.domain_id, delta.delta_type)
                == (
                    presentation::STATE_DOMAIN_WORLD_PRESENTATION,
                    presentation::DELTA_TYPE_WORLD_PRESENTATION_BATCH_V1,
                )
            {
                return Ok(delta.payload.to_vec());
            }
        }
    }
    Err("C18a original damage presentation absent".into())
}

async fn first_cast_vitals(mut inbound: &[u8]) -> TestResult<(u64, Vec<u8>)> {
    use oteryn_protocol_oteryn::{MessageType, decode_state_delta, decode_wire_envelope};
    while !inbound.is_empty() {
        let frame = oteryn_dev_client::native_session::read_frame(&mut inbound).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::StateDelta {
            let delta = decode_state_delta(envelope.payload())?;
            if delta.domain_id == actor_spell::STATE_DOMAIN_ACTOR_VITALS {
                if delta.delta_type != actor_spell::DELTA_TYPE_ACTOR_VITALS_V1 {
                    return Err("C18a unexpected stats delta type".into());
                }
                return Ok((envelope.server_sequence(), delta.payload.to_vec()));
            }
        }
    }
    Err("C18a first cast stats frame absent".into())
}

fn collect_vitals(
    client: &mut oteryn_dev_client::DevClientSession,
    history: Option<&mut Vec<(u64, u32, u32)>>,
) {
    let events = client.take_events();
    if let Some(history) = history {
        history.extend(events.into_iter().filter_map(|event| match event {
            SessionEvent::ActorVitals(delta) => {
                Some((delta.server_sequence, delta.value.mana, delta.value.soul))
            }
            _ => None,
        }));
    }
}

async fn wait_for_cast_tile(
    client: &mut oteryn_dev_client::DevClientSession,
    mut vitals: Option<&mut Vec<(u64, u32, u32)>>,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    loop {
        let entities = client
            .world_entities()
            .ok_or("C18a cast tile occupancy unavailable")?;
        let occupant = entities.iter().find(|entity| {
            matches!(
                entity.kind,
                oteryn_dev_client::EntityKind::Creature
                    | oteryn_dev_client::EntityKind::Npc
                    | oteryn_dev_client::EntityKind::Player
            ) && tile(entity.position).ok() == Some(CAST_TILE)
        });
        let Some(occupant) = occupant else {
            return Ok(());
        };
        if tokio::time::Instant::now() >= deadline {
            return Err(
                format!("C18a fixed cast tile is occupied by {:?}", occupant.entity).into(),
            );
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        collect_vitals(client, vitals.as_deref_mut());
    }
}
