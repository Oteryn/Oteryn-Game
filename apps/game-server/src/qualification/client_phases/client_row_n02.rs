//! N02 client replay capture: a real accepted Wheel allocation Save (and the current-state query
//! before it) for the named-build, export and import path. Client owner:
//! `apps/client/src/features/0148-wheel-allocation/presets.rs` over client-ui `settings.rs`.
//!
//! The Save writes the core slot, the replies are captured, then the original allocation is
//! restored on the dedicated character after verifying its durable state.
use super::*;
use oteryn_dev_client::client_runtime::wheel_allocation::WheelResponse;
use oteryn_dev_client::{DevClientSession, EntityRef};
use oteryn_protocol_oteryn::wheel;

const CORE_SLOT: usize = 20;

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    use std::io::Write;
    let Some(root) =
        std::env::var_os("N02_CAPTURE_DIR").or_else(|| std::env::var_os("E2E_ALL_CAPTURE_DIR"))
    else {
        return Ok(());
    };
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(Path::new(&root).join(name))?
        .write_all(bytes)?;
    Ok(())
}

fn request(action: u8, occurrence: [u8; 16], revision: u64, points: [u16; 36]) -> Vec<u8> {
    wheel::encode_wheel_request(&wheel::WheelAllocationRequest {
        action,
        occurrence,
        expected_revision: revision,
        points,
        vessels: None,
    })
    .unwrap_or_default()
}

async fn send(
    client: &mut DevClientSession,
    payload: Vec<u8>,
    name: &str,
    keep: bool,
) -> TestResult<(WheelResponse, wheel::WheelAllocationReply)> {
    if payload.is_empty() {
        return Err(format!("{name} request did not encode").into());
    }
    use oteryn_protocol_oteryn::atelier::{resources, v2};
    let before_sequence = client.last_server_sequence();
    let session = client.session_mut();
    let result = session
        .gated_command(
            wheel::CAPABILITY_WHEEL_ALLOCATION_V1,
            wheel::COMMAND_TYPE_WHEEL_ALLOCATION,
            &payload,
        )
        .await?;
    let current = session
        .gated_command(
            wheel::CAPABILITY_WHEEL_ALLOCATION_V1,
            wheel::COMMAND_TYPE_WHEEL_ALLOCATION,
            &[8, 0],
        )
        .await?;
    let atelier = session
        .gated_command(
            v2::CAPABILITY_GEM_ATELIER_V2,
            v2::COMMAND_TYPE_ATELIER_V2_QUERY,
            &v2::encode_query(),
        )
        .await?;
    let resources = session
        .gated_command(
            resources::CAPABILITY_ATELIER_RESOURCES_V1,
            resources::COMMAND_TYPE_ATELIER_RESOURCES_QUERY,
            &resources::encode_query(),
        )
        .await?;
    for (frame, status) in [
        ("receipt", result.status),
        ("wheel", current.status),
        ("atelier", atelier.status),
        ("resources", resources.status),
    ] {
        if status != CommandStatus::Accepted {
            return Err(format!("{name} {frame} status={status:?} expected Accepted").into());
        }
    }
    let response = WheelResponse {
        kind: wheel::COMMAND_TYPE_WHEEL_ALLOCATION,
        status: result.status,
        payload: result.payload,
        wheel: Some(current.payload),
        atelier: Some(atelier.payload),
        resources: Some(resources.payload),
    };
    let receipt = wheel::decode_wheel_reply(&response.payload)
        .map_err(|error| format!("{name} receipt: {error:?}"))?;
    let current = response
        .wheel
        .as_deref()
        .ok_or("current Wheel frame absent")?;
    let current = wheel::decode_wheel_reply(current)
        .map_err(|error| format!("{name} current Wheel: {error:?}"))?;
    if response.atelier.is_none() || (!receipt.refused && receipt != current) {
        return Err(format!("{name} frames inconsistent: {receipt:?} {current:?}").into());
    }
    if keep {
        let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
        let sequence = client.last_server_sequence();
        if sequence <= before_sequence {
            return Err("Wheel response did not advance the server sequence".into());
        }
        let atelier_v2 = client
            .selected_capabilities()
            .contains(&oteryn_protocol_oteryn::atelier::v2::CAPABILITY_GEM_ATELIER_V2);
        capture(&format!("n02-{name}-request.bin"), &payload)?;
        let atelier = atelier_view(&response)?;
        use oteryn_protocol_oteryn::wheel_presentation as diagram;
        let request = diagram::Request {
            nonce: 1,
            wheel_revision: receipt.revision,
            atelier_revision: atelier.revision,
            points: receipt.points,
            vessels: [None; 4],
        };
        let payload = request
            .encode()
            .ok_or("N02 Wheel presentation request did not encode")?;
        let display = client
            .session_mut()
            .gated_command(
                wheel::CAPABILITY_WHEEL_ALLOCATION_V1,
                diagram::COMMAND_TYPE_WHEEL_PRESENTATION,
                &payload,
            )
            .await?;
        let view = diagram::Reply::decode(&display.payload)
            .ok_or("N02 invalid Wheel presentation reply")?;
        if display.status != CommandStatus::Accepted
            || view.request != request
            || view.available != 25
        {
            return Err(format!(
                "N02 Wheel presentation status={:?} available={} expected_available=25",
                display.status, view.available
            )
            .into());
        }
        capture(&format!("n02-{name}-diagram.bin"), &display.payload)?;
        let capture_sequence = client.last_server_sequence();
        capture(
            &format!("n02-{name}-wire.txt"),
            format!(
                "{session} {capture_sequence} {} {} {}\n",
                response.kind,
                response.status as u32,
                u8::from(atelier_v2)
            )
            .as_bytes(),
        )?;
        capture(&format!("n02-{name}-receipt.bin"), &response.payload)?;
        for (suffix, frame) in [
            ("wheel", &response.wheel),
            ("atelier", &response.atelier),
            ("resources", &response.resources),
        ] {
            if let Some(bytes) = frame {
                capture(&format!("n02-{name}-{suffix}.bin"), bytes)?;
            }
        }
    }
    Ok((response, receipt))
}

fn csv(points: &[u16; 36]) -> String {
    points
        .iter()
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

async fn journey(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult<String> {
    character_prep::declare_seeded(
        "N02.client",
        "level-75,premium,promotion-gold",
        "Wheel Save",
    );
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_N02_ACCOUNT").await?;
    let (accounts, characters) = dedicated.replace_slot(clients, 1)?;
    let clients = &SeamClients {
        accounts: &accounts,
        characters: &characters,
        ..*clients
    };
    let mut prepare = connect_world(clients, 0xad).await?;
    check_stats(&mut prepare, "knight", false).await?;
    request_fixture_control(
        uuid_text(prepare.join_snapshot().game_session_id.as_bytes()),
        "--profile",
        "n02-knight".into(),
    )
    .await?;
    character_prep::grant_item(&prepare, 3043, 2).await?;
    logout(&mut prepare).await?;
    drop(prepare);
    let mut client = connect_world(clients, 0xad).await?;
    let start = tile(client.world_spatial().actor_position)?;
    check_stats(&mut client, "knight", true).await?;
    cross_castle_door(&mut client, world, StepDirection::North).await?;
    let placement = TilePos {
        x: 32311,
        y: 32172,
        floor: -6,
    };
    let npc = npcs
        .npcs
        .iter()
        .find(|npc| npc.key == "oteryn:npc.king_tibianus")
        .ok_or("missing King Tibianus placement")?;
    if !npc
        .placements
        .iter()
        .any(|p| (p.x, p.y, p.floor) == (placement.x, placement.y, placement.floor))
    {
        return Err("King Tibianus is absent from his authored placement".into());
    }
    walk_route(&mut client, world, |at| {
        at.floor == placement.floor
            && at.x.abs_diff(placement.x).max(at.y.abs_diff(placement.y)) <= 3
    })
    .await?;
    let king = client
        .world_entities()
        .and_then(|view| {
            view.iter().find(|actor| {
                actor.kind == oteryn_dev_client::EntityKind::Npc
                    && tile(actor.position).ok() == Some(placement)
            })
        })
        .ok_or("King Tibianus was not visible at his authored placement")?
        .entity;
    character_prep::grant_premium(&client).await?;
    talk(
        &mut client,
        king,
        "Hail King",
        "I greet thee, my loyal subject.",
    )
    .await?;
    talk(
        &mut client,
        king,
        "promotion",
        "Do you want to be promoted for 20000 gold coins?",
    )
    .await?;
    check_gold(&mut client, 20_050).await?;
    talk(&mut client, king, "yes", "You are now promoted.").await?;
    check_gold(&mut client, 50).await?;
    check_stats(&mut client, "elite_knight", true).await?;
    cross_castle_door(&mut client, world, StepDirection::South).await?;
    walk_route(&mut client, world, |at| at == start).await?;
    if !client
        .selected_capabilities()
        .contains(&wheel::CAPABILITY_WHEEL_ALLOCATION_V1)
    {
        return Err("admission omitted Wheel capability 148".into());
    }
    require_protection_zone(&client, world)?;
    let (_, initial) = send(&mut client, vec![8, 0], "query", true).await?;
    let query_sequence = client.last_server_sequence();
    if initial.refused || initial.revision != 0 || initial.points != [0; 36] {
        return Err(format!("query refused: {initial:?}").into());
    }
    let mut target = initial.points;
    target[CORE_SLOT] = if initial.points[CORE_SLOT] == 25 {
        0
    } else {
        25
    };
    let (saved_frames, saved) = send(
        &mut client,
        request(1, v7(0xad, 1), initial.revision, target),
        "save",
        true,
    )
    .await?;
    if saved.refused || saved.revision != initial.revision + 1 || saved.points != target {
        return Err(format!("save was not accepted exactly: {saved:?}").into());
    }
    let save_sequence = client.last_server_sequence();
    let expected = format!(
        "revision={}\npoints={}\nsaved_revision={}\nsaved_points={}\n",
        initial.revision,
        csv(&initial.points),
        saved.revision,
        csv(&saved.points)
    );
    let session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut client = connect_world(clients, 0xad).await?;
    let (durable_frames, durable) = send(&mut client, vec![8, 0], "relog", false).await?;
    if client.join_snapshot().game_session_id == session || durable != saved {
        return Err(format!("fresh Wheel differs from saved allocation: {durable:?}").into());
    }
    check_gold(&mut client, 50).await?;
    check_stats(&mut client, "elite_knight", true).await?;
    require_protection_zone(&client, world)?;
    let saved_atelier = atelier_view(&saved_frames)?;
    let durable_atelier = atelier_view(&durable_frames)?;
    if saved_atelier.refused
        || durable_atelier.refused
        || saved_atelier.wheel_revision != saved.revision
        || durable_atelier.wheel_revision != durable.revision
        || saved_atelier.gems != durable_atelier.gems
        || saved_atelier.vessels != durable_atelier.vessels
        || saved_atelier.vessel_nodes != durable_atelier.vessel_nodes
        || durable_atelier.vessels.len() != 8
        || durable_atelier.vessel_nodes.len() != 8
        || !durable_atelier.premium
        || durable_atelier
            .vessels
            .iter()
            .any(|vessel| vessel.gem.is_some())
    {
        return Err(format!("N02 relog Atelier mismatch: saved_slots={} durable_slots={} durable_nodes={} durable_premium={} expected_slots=8 expected_nodes=8 expected_premium=true", saved_atelier.vessels.len(), durable_atelier.vessels.len(), durable_atelier.vessel_nodes.len(), durable_atelier.premium).into());
    }
    let (_, restored) = send(
        &mut client,
        request(1, v7(0xad, 2), saved.revision, initial.points),
        "restore",
        false,
    )
    .await?;
    if restored.refused
        || restored.points != initial.points
        || restored.revision != saved.revision + 1
    {
        return Err(format!("Wheel restore was not accepted exactly: {restored:?}").into());
    }
    logout(&mut client).await?;
    capture("n02-expected.txt", expected.as_bytes())?;
    Ok(format!(
        "live_session={} query_sequence={query_sequence} save_sequence={save_sequence} evidence=revision={}->{}|core_slot={}->{}",
        uuid_text(session.as_bytes()),
        initial.revision,
        saved.revision,
        initial.points[CORE_SLOT],
        saved.points[CORE_SLOT]
    ))
}

async fn check_stats(
    client: &mut DevClientSession,
    expected_vocation: &str,
    seeded: bool,
) -> TestResult {
    let stats = client
        .session_mut()
        .gated_command(
            oteryn_protocol_oteryn::cyclopedia_character::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            388,
            &[
                oteryn_protocol_oteryn::cyclopedia_character::CharacterTab::GeneralStats as u8,
                0,
                0,
                1,
            ],
        )
        .await?;
    let selection = general_stats::selected(client);
    let vocation = general_stats::validate(&stats.payload, selection)?;
    let expected = oteryn_protocol_oteryn::cyclopedia_character::vocation_id(expected_vocation)
        .ok_or("unknown N02 expected vocation")?;
    if vocation != Some(expected) {
        return Err(format!(
            "N02 vocation expected {expected_vocation} ({expected}), got {vocation:?}"
        )
        .into());
    }
    let level = u32::from_le_bytes(stats.payload[9..13].try_into()?);
    let xp = u64::from_le_bytes(stats.payload[1..9].try_into()?);
    if stats.status != CommandStatus::Accepted || (seeded && (level != 75 || xp != 6_489_800)) {
        return Err(format!("seeded stats level={level} xp={xp}").into());
    }
    Ok(())
}

fn require_protection_zone(client: &DevClientSession, world: &BundleWorld) -> TestResult {
    let at = tile(client.world_spatial().actor_position)?;
    if !world
        .base()
        .tile(at.x, at.y, at.floor)
        .is_some_and(|tile| tile.protection_zone())
    {
        return Err(format!("N02 Wheel mutation outside protection zone at {at:?}").into());
    }
    Ok(())
}

fn atelier_view(
    response: &WheelResponse,
) -> TestResult<oteryn_protocol_oteryn::atelier::v2::AtelierReply> {
    oteryn_protocol_oteryn::atelier::v2::decode_reply(
        response
            .atelier
            .as_deref()
            .ok_or("missing Atelier v2 frame")?,
    )
    .map_err(|error| format!("N02 Atelier v2 decode: {error:?}").into())
}

async fn check_gold(client: &mut DevClientSession, expected_carried: u64) -> TestResult {
    use oteryn_protocol_oteryn::atelier::resources as wire;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_ATELIER_RESOURCES_V1,
            wire::COMMAND_TYPE_ATELIER_RESOURCES_QUERY,
            &wire::encode_query(),
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("N02 balance status={:?} expected Accepted", reply.status).into());
    }
    let balance = wire::decode_reply(&reply.payload)
        .map_err(|error| format!("N02 balance decode: {error:?}"))?;
    if (balance.carried_gold, balance.bank_gold) != (expected_carried, 0) {
        return Err(format!(
            "N02 balance carried={} bank={} expected_carried={expected_carried} expected_bank=0",
            balance.carried_gold, balance.bank_gold
        )
        .into());
    }
    Ok(())
}

async fn talk(
    client: &mut DevClientSession,
    npc: EntityRef,
    text: &str,
    expected: &str,
) -> TestResult {
    use oteryn_protocol_oteryn::npc_service as wire;
    tokio::time::sleep(Duration::from_millis(260)).await;
    let payload = wire::encode_npc_talk_intent(&wire::NpcTalkIntent {
        npc_actor: npc,
        text: text.into(),
    })
    .map_err(|error| format!("N02 {text:?} talk encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_NPC_SERVICE_V1,
            wire::COMMAND_TYPE_NPC_TALK_INTENT,
            &payload,
        )
        .await?;
    let disposition = wire::decode_npc_intent_result(&reply.payload)
        .map_err(|error| format!("N02 {text:?} talk decode: {error:?}"))?;
    if reply.status != CommandStatus::Accepted || disposition != wire::NpcIntentDisposition::Ok {
        return Err(format!(
            "N02 {text:?} talk status={:?} disposition={disposition:?}",
            reply.status
        )
        .into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1)
        .await?;
    let conversation = wire::decode_npc_conversation(&delta.payload)
        .map_err(|error| format!("N02 {text:?} conversation decode: {error:?}"))?;
    if delta.server_sequence <= reply.server_sequence
        || conversation.open != Some((npc, wire::NpcConversationState::Open))
        || conversation.lines != [expected]
    {
        return Err(format!("N02 {text:?} expected {expected:?}, got {conversation:?}").into());
    }
    Ok(())
}

async fn step_once(
    client: &mut DevClientSession,
    direction: StepDirection,
    expected: TilePos,
) -> TestResult {
    // Each request waits for its move receipt before the next step. The server
    // buffers that next step until its own pacing clock is due.
    let result = client.step(direction).await?;
    if result.status != CommandStatus::Accepted
        || result.disposition != StepDisposition::Moved
        || result.world_spatial_delta.is_none()
        || tile(client.world_spatial().actor_position)? != expected
    {
        return Err(format!("castle step {direction:?} expected {expected:?}: {result:?}").into());
    }
    verdict::Harness::global().progress("walk");
    client.take_events();
    Ok(())
}

async fn walk_route(
    client: &mut DevClientSession,
    world: &BundleWorld,
    goal: impl Fn(TilePos) -> bool,
) -> TestResult {
    let mut at = tile(client.world_spatial().actor_position)?;
    let path = route(world, at, goal).ok_or_else(|| format!("castle route absent from {at:?}"))?;
    for direction in path {
        at = destination(world, at, direction).ok_or("missing planned castle landing")?;
        step_once(client, direction, at).await?;
    }
    Ok(())
}

async fn cross_castle_door(
    client: &mut DevClientSession,
    world: &BundleWorld,
    direction: StepDirection,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    const DOOR: TilePos = TilePos {
        x: 32314,
        y: 32180,
        floor: -7,
    };
    let (from, to) = match direction {
        StepDirection::North => (32181, 32179),
        StepDirection::South => (32179, 32181),
        _ => return Err("invalid castle doorway direction".into()),
    };
    let doors = world.facts().doors();
    let mut keys = doors
        .keys()
        .filter(|&key| crate::map::view::placement_of(key).is_some_and(|(at, _)| at == DOOR));
    let key = keys.next().ok_or("missing fixed castle door")?;
    if keys.next().is_some() || doors.authored_closed(key).is_none() {
        return Err("ambiguous or unsupported fixed castle door".into());
    }
    let (_, ordinal) =
        crate::map::view::placement_of(key).ok_or("invalid castle door placement")?;
    let appearance = world
        .base()
        .tile(DOOR.x, DOOR.y, DOOR.floor)
        .and_then(|tile| tile.appearance(ordinal));
    if appearance != Some(1632) {
        return Err(
            format!("N02 castle door at {DOOR:?} appearance={appearance:?} expected=1632").into(),
        );
    }
    walk_route(client, world, |at| at == TilePos { y: from, ..DOOR }).await?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    // The ordinary door stays open after the committed northbound USE.
    // The return STEP proves that state directly through the modern map session.
    if direction == StepDirection::North {
        let result = client.use_object(&placement, 0).await?;
        if result.status != CommandStatus::Accepted
            || result.disposition != UseDisposition::Committed
        {
            return Err(format!("castle door at {DOOR:?} item=1632 refused: {result:?}").into());
        }
    }
    step_once(client, direction, DOOR).await?;
    step_once(client, direction, TilePos { y: to, ..DOOR }).await
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err("N02 logout refused".into());
    }
    Ok(())
}

pub(super) async fn run_packet(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) {
    let started = std::time::Instant::now();
    // The paced castle round trip exceeds 120 seconds without any refused steps.
    match verdict::indexed_timeout(
        Duration::from_secs(360),
        verdict::observe(journey(clients, world, npcs)),
    )
    .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=N02.client status=CAPTURED seconds={:.3} {observed}",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=N02.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=N02.client status=FAIL evidence=timeout"),
    }
}
