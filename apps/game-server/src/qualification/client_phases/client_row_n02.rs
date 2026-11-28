//! N02 client replay capture: a real accepted Wheel allocation Save (and the current-state query
//! before it) for the named-build, export and import path. Client owner:
//! `apps/client/src/features/0148-wheel-allocation/presets.rs` over client-ui `settings.rs`.
//!
//! The Save writes the core slot, the replies are captured, then the original allocation is
//! restored so later rows start from the same Wheel.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::wheel_allocation::{self, WheelCommand, WheelResponse};
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
    let command = WheelCommand::new(wheel::COMMAND_TYPE_WHEEL_ALLOCATION, payload)
        .ok_or("Wheel runtime rejected the command")?;
    let before_sequence = client.last_server_sequence();
    let response = wheel_allocation::send(client.session_mut(), &command).await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!("{name} status={:?}", response.status).into());
    }
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
        capture(
            &format!("n02-{name}-wire.txt"),
            format!(
                "{session} {sequence} {} {} {}\n",
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
    let selection = general_stats::selected(&client);
    general_stats::validate(&stats.payload, selection)?;
    let level = u32::from_le_bytes(stats.payload[9..13].try_into()?);
    let xp = u64::from_le_bytes(stats.payload[1..9].try_into()?);
    if stats.status != CommandStatus::Accepted || level != 75 || xp != 6_489_800 {
        return Err(format!("seeded stats level={level} xp={xp}").into());
    }
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
    let (_, greeting) = client_row_p20::talk(&mut client, king, "hi").await?;
    if greeting.is_empty() {
        return Err("King Tibianus greeting was empty".into());
    }
    let (_, quote) = client_row_p20::talk(&mut client, king, "promotion").await?;
    if quote != ["Do you want to be promoted for 20000 gold coins?"] {
        return Err(format!("King Tibianus promotion quote: {quote:?}").into());
    }
    let unpaid = gold(&mut client).await?;
    if unpaid != 20_050 {
        return Err(format!("N02 pre-promotion gold was {unpaid}, expected 20050").into());
    }
    let (_, confirmation) = client_row_p20::talk(&mut client, king, "yes").await?;
    if confirmation != ["You are now promoted."] {
        return Err(format!("King Tibianus promotion confirmation: {confirmation:?}").into());
    }
    let paid = gold(&mut client).await?;
    if paid != unpaid - 20_000 {
        return Err(format!(
            "N02 promotion charged {} -> {paid}, expected {unpaid} -> 50",
            unpaid
        )
        .into());
    }
    cross_castle_door(&mut client, world, StepDirection::South).await?;
    walk_route(&mut client, world, |at| at == start).await?;
    if !client
        .selected_capabilities()
        .contains(&wheel::CAPABILITY_WHEEL_ALLOCATION_V1)
    {
        return Err("admission omitted Wheel capability 148".into());
    }
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
    let (_, saved) = send(
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
    let (_, durable) = send(&mut client, vec![8, 0], "relog", false).await?;
    if client.join_snapshot().game_session_id == session || durable != saved {
        return Err(format!("fresh Wheel differs from saved allocation: {durable:?}").into());
    }
    if gold(&mut client).await? != paid {
        return Err("N02 promotion debit did not survive relog".into());
    }
    // The last session's logout releases the disposable Premium view. Prepare
    // the fresh session only after proving the saved Wheel survived admission.
    character_prep::grant_premium(&client).await?;
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

async fn gold(client: &mut DevClientSession) -> TestResult<u64> {
    let balance = oteryn_dev_client::client_runtime::npc_shop::balances(client.session_mut())
        .await?
        .ok_or("N02 live gold balance absent")?;
    balance
        .bank_gold
        .checked_add(balance.carried_gold)
        .ok_or_else(|| "N02 gold balance overflow".into())
}

async fn step_once(
    client: &mut DevClientSession,
    direction: StepDirection,
    expected: TilePos,
) -> TestResult {
    let result = client.step(direction).await?;
    if result.status != CommandStatus::Accepted
        || result.disposition != StepDisposition::Moved
        || result.world_spatial_delta.is_none()
        || tile(client.world_spatial().actor_position)? != expected
    {
        return Err(format!("castle step {direction:?} expected {expected:?}: {result:?}").into());
    }
    verdict::Harness::global().progress("walk");
    tokio::time::sleep(Duration::from_millis(u64::from(result.step_wait_ms))).await;
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
