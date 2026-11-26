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
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
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
    let response = wheel_allocation::send(client.session_mut(), &command).await?;
    if !matches!(
        response.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
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

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
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
    character_prep::grant_premium(&prepare).await?;
    logout(&mut prepare).await?;
    drop(prepare);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
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
    let king = full_row_p20::approach_npc(
        &mut client,
        world,
        &metadata.npcs,
        "oteryn:npc.king_tibianus",
    )
    .await?;
    let greeting = talk(&mut client, king, "hi").await?;
    if greeting.is_empty()
        || talk(&mut client, king, "promotion").await?
            != ["Do you want to be promoted for 20000 gold coins?"]
        || talk(&mut client, king, "yes").await? != ["You are now promoted."]
    {
        return Err("King Tibianus did not confirm promotion".into());
    }
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at == start
    })
    .ok_or("no route back to the Wheel starting tile")?;
    walk(&mut client, world, &path).await?;
    if !client
        .selected_capabilities()
        .contains(&wheel::CAPABILITY_WHEEL_ALLOCATION_V1)
    {
        return Err("admission omitted Wheel capability 148".into());
    }
    let (_, initial) = send(&mut client, vec![8, 0], "query", true).await?;
    if initial.refused {
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
        "revision={}->{}|core_slot={}->{}",
        initial.revision, saved.revision, initial.points[CORE_SLOT], saved.points[CORE_SLOT]
    ))
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err("N02 logout refused".into());
    }
    Ok(())
}

async fn talk(
    client: &mut DevClientSession,
    actor: oteryn_dev_client::EntityRef,
    text: &str,
) -> TestResult<Vec<String>> {
    use oteryn_protocol_oteryn::npc_service as wire;
    let disposition = oteryn_dev_client::client_runtime::npc_shop::talk(
        client.session_mut(),
        &wire::NpcTalkIntent {
            npc_actor: actor,
            text: text.into(),
        },
    )
    .await?;
    if disposition != wire::NpcIntentDisposition::Ok {
        return Err(format!("NPC {text} refused: {disposition:?}").into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1)
        .await?;
    let conversation = wire::decode_npc_conversation(&delta.payload)
        .map_err(|error| format!("promotion conversation: {error:?}"))?;
    if conversation.open.map(|(speaker, _)| speaker) != Some(actor) {
        return Err("promotion reply from a different NPC".into());
    }
    Ok(conversation.lines)
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=N02.client status=PASS seconds={:.3} evidence={observed}",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=N02.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=N02.client status=FAIL evidence=timeout"),
    }
}
