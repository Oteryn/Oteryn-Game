//! P20 client replay capture: the real King Tibianus promotion dialogue (greeting, the 20000 gold
//! quote and the confirmation reply of a character that cannot be promoted yet). The consumer is
//! `DialoguePanel::request`/`apply` in `crates/client-ui/src/features/0003-npc-service`.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, EntityRef};
use oteryn_protocol_oteryn::npc_service as wire;
use std::io::Write;

pub(super) fn capture(row: &str, name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "{row}_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

/// Walks next to the authored NPC `key` and returns its visible actor.
pub(super) async fn approach_npc(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    key: &str,
) -> TestResult<EntityRef> {
    let npc = npcs
        .npcs
        .iter()
        .find(|npc| npc.key == key)
        .ok_or_else(|| format!("{key} absent from the compiled world"))?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        npc.placements.iter().any(|place| {
            place.floor == at.floor && at.x.abs_diff(place.x).max(at.y.abs_diff(place.y)) <= 3
        })
    })
    .ok_or_else(|| format!("{key} unreachable"))?;
    walk(client, world, &path).await?;
    client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entity| {
                entity.kind == EntityKind::Npc
                    && npc.placements.iter().any(|place| {
                        entity.position.x == i32::from(place.x)
                            && entity.position.y == i32::from(place.y)
                            && entity.position.floor == -i16::from(place.floor)
                    })
            })
        })
        .map(|entity| entity.entity)
        .ok_or_else(|| format!("{key} not visible after walking to it").into())
}

/// Sends one talk intent and returns the raw conversation payload with its decoded lines.
pub(super) async fn talk(
    client: &mut DevClientSession,
    npc: EntityRef,
    text: &str,
) -> TestResult<(Vec<u8>, Vec<String>)> {
    let (payload, lines, _) = capture_talk(client, npc, text).await?;
    Ok((payload, lines))
}

async fn talk_command(
    client: &mut DevClientSession,
    npc: EntityRef,
    text: &str,
) -> TestResult<(CommandStatus, wire::NpcIntentDisposition, u64)> {
    tokio::time::sleep(Duration::from_millis(260)).await;
    let payload = wire::encode_npc_talk_intent(&wire::NpcTalkIntent {
        npc_actor: npc,
        text: text.into(),
    })
    .map_err(|error| format!("{text} encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_NPC_SERVICE_V1,
            wire::COMMAND_TYPE_NPC_TALK_INTENT,
            &payload,
        )
        .await?;
    let disposition = wire::decode_npc_intent_result(&reply.payload)
        .map_err(|error| format!("{text} reply decode: {error:?}"))?;
    Ok((reply.status, disposition, reply.server_sequence))
}

fn expect_talk_result(
    text: &str,
    status: CommandStatus,
    disposition: wire::NpcIntentDisposition,
    expected: wire::NpcIntentDisposition,
) -> TestResult<()> {
    if status != CommandStatus::Accepted || disposition != expected {
        return Err(format!(
            "talk {text:?} observed={status:?}/{disposition:?} expected=Accepted/{expected:?}"
        ).into());
    }
    Ok(())
}

async fn capture_talk(
    client: &mut DevClientSession,
    npc: EntityRef,
    text: &str,
) -> TestResult<(Vec<u8>, Vec<String>, u64)> {
    let (status, disposition, reply_sequence) = talk_command(client, npc, text).await?;
    expect_talk_result(text, status, disposition, wire::NpcIntentDisposition::Ok)?;
    loop {
        let delta = client
            .session_mut()
            .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1)
            .await?;
        if delta.server_sequence <= reply_sequence {
            continue;
        }
        let conversation = wire::decode_npc_conversation(&delta.payload)
            .map_err(|error| format!("{text} conversation decode: {error:?}"))?;
        if conversation.open != Some((npc, wire::NpcConversationState::Open)) {
            return Err(format!("{text} delta belonged to another NPC").into());
        }
        if conversation
            .lines
            .iter()
            .any(|line| line.contains(['|', '\n']))
        {
            return Err("conversation line contains a reserved separator".into());
        }
        return Ok((delta.payload, conversation.lines, delta.server_sequence));
    }
}

pub(super) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

async fn step_to(
    client: &mut DevClientSession,
    direction: StepDirection,
    expected: TilePos,
) -> TestResult {
    let outcome = client.step(direction).await?;
    if outcome.status != CommandStatus::Accepted
        || outcome.disposition != StepDisposition::Moved
        || outcome.world_spatial_delta.is_none()
        || tile(client.world_spatial().actor_position)? != expected
    {
        return Err(format!(
            "P20 castle step {direction:?} expected {expected:?}, observed {outcome:?}"
        )
        .into());
    }
    verdict::Harness::global().progress("walk");
    client
        .session_mut()
        .service_liveness_discarding_events(Duration::from_millis(u64::from(outcome.step_wait_ms)))
        .await?;
    Ok(())
}

async fn walk_castle(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    for &direction in path {
        let from = tile(client.world_spatial().actor_position)?;
        let expected =
            destination(world, from, direction).ok_or("P20 castle route lost its landing")?;
        step_to(client, direction, expected).await?;
    }
    Ok(())
}

async fn verify_open_door(
    client: &mut DevClientSession,
    world: &BundleWorld,
    door: TilePos,
    after_sequence: u64,
    revision: u64,
) -> TestResult {
    use oteryn_dev_client::SessionEvent;
    use oteryn_protocol_oteryn::world_map::MapOrigin;

    let position = spatial_basis::current(client)?.position(door);
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            for event in client.wait_for_events(Duration::from_secs(5)).await? {
                let SessionEvent::WorldMap(delta) = event else {
                    continue;
                };
                if delta.server_sequence > after_sequence
                    && delta.value.header.bundle_digest == world.base().digest()
                    && let Some(tile) = delta.value.tiles.iter().find(|tile| tile.position == position)
                {
                    if tile.items.iter().any(|item| {
                        item.appearance_id == 1633
                            && item.origin == MapOrigin::BaseOrdinal { ordinal: 1, object_revision: revision }
                    }) {
                        return Ok(());
                    }
                    return Err(format!("P20 castle door map tile did not show the open appearance and revision {revision}: {tile:?}").into());
                }
            }
        }
    }).await;
    result.map_err(|_| "P20 castle door opening did not publish its map delta")?
}

async fn approach_king(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult<EntityRef> {
    use crate::map::{door::AuthoredClosedDoor, view::placement_key};
    use oteryn_protocol_oteryn::world_object::UseDisposition;

    // Enter the authored public castle door from its south side and take the stairs.
    const DOOR: TilePos = TilePos {
        x: 32313,
        y: 32180,
        floor: -7,
    };
    const APPROACH: TilePos = TilePos { y: 32181, ..DOOR };
    let key = placement_key(DOOR, 1).ok_or("P20 castle door placement key")?;
    let doors = world.facts().doors();
    if doors.authored_closed(key) != Some(AuthoredClosedDoor::KeyUnlocked) {
        return Err("P20 authored castle door is absent or no longer unlocked".into());
    }
    let king = npcs
        .npcs
        .iter()
        .find(|npc| npc.key == "oteryn:npc.king_tibianus")
        .ok_or("King Tibianus absent from the compiled world")?;
    let approach = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at == APPROACH
    })
    .ok_or("P20 castle door unreachable from the admitted player position")?;
    let stairs = route(world, DOOR, |at| {
        king.placements.iter().any(|place| {
            place.floor == at.floor && at.x.abs_diff(place.x).max(at.y.abs_diff(place.y)) <= 3
        })
    })
    .ok_or("King Tibianus unreachable from the authored castle entrance")?;
    walk_castle(client, world, &approach).await?;

    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let revision = doors.revision(key);
    let used = client.use_object(&placement, revision).await?;
    if used.status != CommandStatus::Accepted || used.disposition != UseDisposition::Committed {
        return Err(format!(
            "P20 castle door did not open on its first use: {:?}/{:?}",
            used.status, used.disposition
        )
        .into());
    }
    verify_open_door(
        client,
        world,
        DOOR,
        used.result_server_sequence,
        revision + 1,
    )
    .await?;
    step_to(client, StepDirection::North, DOOR).await?;
    walk_castle(client, world, &stairs).await?;
    client
        .world_entities()
        .and_then(|view| {
            view.iter().find(|entity| {
                entity.kind == EntityKind::Npc
                    && king.placements.iter().any(|place| {
                        entity.position.x == i32::from(place.x)
                            && entity.position.y == i32::from(place.y)
                            && entity.position.floor == -i16::from(place.floor)
                    })
            })
        })
        .map(|entity| entity.entity)
        .ok_or_else(|| "King Tibianus not visible after walking up the castle stairs".into())
}

fn expect_royal_greeting(lines: &[String]) -> TestResult<()> {
    if lines != ["I greet thee, my loyal subject."] {
        return Err(format!("Hail King! greeting observed={lines:?}").into());
    }
    Ok(())
}

fn expect_unpromoted(stage: &str, purchase: Option<String>) -> TestResult<()> {
    if let Some(occurrence) = purchase {
        return Err(format!("P20 {stage} found durable promotion occurrence={occurrence}").into());
    }
    Ok(())
}

#[test]
fn p20_rejects_audit_false_positives() {
    use wire::NpcIntentDisposition::{NoConversation, Ok};
    assert!(expect_talk_result("hi", CommandStatus::Accepted, Ok, NoConversation).is_err());
    assert!(expect_talk_result("Hail King!", CommandStatus::Rejected, Ok, Ok).is_err());
    assert!(expect_talk_result("Hail King!", CommandStatus::Accepted, Ok, Ok).is_ok());
    assert!(expect_royal_greeting(&["Greetings, Packet One.".into()]).is_err());
    assert!(expect_royal_greeting(&["I greet thee, my loyal subject.".into()]).is_ok());
    assert!(expect_unpromoted("after relog", Some("illicit-occurrence".into())).is_err());
    assert!(expect_unpromoted("after relog", None).is_ok());
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let capture_dir = required("E2E_ALL_CAPTURE_DIR")?;
    let capture_dir = Path::new(&capture_dir);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_character(clients, 0x52, 0).await?;
    let npc = approach_king(&mut client, world, &metadata.npcs).await?;
    let (status, disposition, _) = talk_command(&mut client, npc, "hi").await?;
    expect_talk_result("hi", status, disposition, wire::NpcIntentDisposition::NoConversation)?;
    let (greeting_bytes, greeting, greeting_sequence) =
        capture_talk(&mut client, npc, "Hail King!").await?;
    expect_royal_greeting(&greeting)?;
    let (quote_bytes, quote, quote_sequence) = capture_talk(&mut client, npc, "promotion").await?;
    if quote != ["Do you want to be promoted for 20000 gold coins?"] {
        return Err(format!("promotion quote observed={quote:?}").into());
    }
    use oteryn_dev_client::client_runtime::npc_shop::balances;
    let funds = balances(client.session_mut()).await?.ok_or("P20 funds absent")?;
    let vocation = super::full_row_p20::vocation(&mut client).await?;
    let session = client.join_snapshot().game_session_id;
    let position = client.world_spatial().actor_position;
    let character = &clients.characters[0];
    expect_unpromoted("before confirmation", super::full_row_p20::promotion_record(clients.url, character).await?)?;
    let (confirm_bytes, confirm, confirm_sequence) = capture_talk(&mut client, npc, "yes").await?;
    if confirm != ["You need current Premium for this service."] {
        return Err(format!("promotion confirmation observed={confirm:?}").into());
    }
    if balances(client.session_mut()).await?.as_ref() != Some(&funds)
        || super::full_row_p20::vocation(&mut client).await? != vocation
        || client.world_spatial().actor_position != position
    {
        return Err("P20 Premium refusal changed funds, vocation, or position".into());
    }
    if !(greeting_sequence < quote_sequence && quote_sequence < confirm_sequence) {
        return Err("P20 conversation sequences did not advance".into());
    }
    let expected = format!(
        "session={}\ngreeting_sequence={greeting_sequence}\nquote_sequence={quote_sequence}\nconfirm_sequence={confirm_sequence}\nnpc_identity={}\nnpc_generation={}\ngreeting={}\nquote={}\nconfirm={}\n",
        hex(session.as_bytes()),
        hex(&npc.identity),
        npc.generation,
        greeting.join("|"),
        quote.join("|"),
        confirm.join("|")
    );
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("P20 client logout refused: {logout:?}").into());
    }
    drop(client);
    let mut fresh = connect_character(clients, 0x53, 0).await?;
    if fresh.join_snapshot().game_session_id == session
        || fresh.world_spatial().actor_position != position
        || balances(fresh.session_mut()).await?.as_ref() != Some(&funds)
        || super::full_row_p20::vocation(&mut fresh).await? != vocation
    {
        return Err("P20 relog did not retain refused character state at the saved position".into());
    }
    expect_unpromoted("after relog", super::full_row_p20::promotion_record(clients.url, character).await?)?;
    let logout = fresh.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("P20 relog logout refused: {logout:?}").into());
    }
    for (name, bytes) in [
        ("p20-greeting.bin", greeting_bytes.as_slice()),
        ("p20-quote.bin", quote_bytes.as_slice()),
        ("p20-confirm.bin", confirm_bytes.as_slice()),
        ("p20-expected.txt", expected.as_bytes()),
    ] {
        std::fs::File::create_new(capture_dir.join(name))?.write_all(bytes)?;
    }
    Ok(expected.replace('\n', ";").replace(' ', "_"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    // The approach from Thais takes over 100 steps paced by the server's receipts.
    match verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P20.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P20.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P20.client status=FAIL evidence=timeout"),
    }
}
