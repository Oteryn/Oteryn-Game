//! S10 client replay capture: today's Rashid placement as an ordinary player. The driver walks
//! to the authored Rashid actor, says `hi` then `trade`, asserts the accepted dispositions, and
//! writes the raw conversation and trade-window delta payloads (`Session::read_gated_delta`)
//! that the session replay consumes (`crates/session/src/tests/e2e_s10.rs`). It buys nothing.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, EntityRef};
use oteryn_protocol_oteryn::npc_service as wire;
use std::io::Write as _;

fn capture(root: &Path, name: &str, bytes: &[u8]) -> TestResult {
    std::fs::File::create_new(root.join(name))
        .and_then(|mut file| file.write_all(bytes))
        .map_err(|error| format!("S10 client capture: write {name}: {error}"))?;
    Ok(())
}

fn identity(npc: EntityRef) -> String {
    let hex: String = npc
        .identity
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!("{hex}:{}", npc.generation)
}

async fn talk(
    client: &mut DevClientSession,
    npc: EntityRef,
    text: &str,
    previous: &mut Option<u64>,
    capture_dir: &Path,
) -> TestResult<(String, u64)> {
    tokio::time::sleep(Duration::from_millis(260)).await;
    let payload = wire::encode_npc_talk_intent(&wire::NpcTalkIntent {
        npc_actor: npc,
        text: text.into(),
    })
    .map_err(|error| format!("step={text} encode: {error:?}"))?;
    let reply = tokio::time::timeout(
        Duration::from_secs(12),
        client.session_mut().gated_command(
            wire::CAPABILITY_NPC_SERVICE_V1,
            wire::COMMAND_TYPE_NPC_TALK_INTENT,
            &payload,
        ),
    )
    .await
    .map_err(|_| format!("step={text} first command reply timed out"))??;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step={text} command status {:?}", reply.status).into());
    }
    let disposition = wire::decode_npc_intent_result(&reply.payload)
        .map_err(|error| format!("step={text} result: {error:?}"))?;
    if disposition != wire::NpcIntentDisposition::Ok {
        return Err(format!("step={text} observed {disposition:?}").into());
    }
    verdict::Harness::global().progress(if text == "hi" {
        "s10-hi-accepted"
    } else {
        "s10-trade-accepted"
    });
    let delta = tokio::time::timeout(Duration::from_secs(12), async {
        loop {
            let delta = client
                .session_mut()
                .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1)
                .await?;
            if delta.server_sequence > reply.server_sequence {
                break Ok::<_, oteryn_dev_client::SessionError>(delta);
            }
        }
    })
    .await;
    let delta = match delta {
        Ok(delta) => delta?,
        Err(_) => {
            let shop = client
                .take_events()
                .into_iter()
                .find_map(|event| match event {
                    oteryn_dev_client::SessionEvent::NpcTradeWindow(delta)
                        if delta.server_sequence > reply.server_sequence =>
                    {
                        Some((
                            delta.value.open.map(|(actor, _)| actor) == Some(npc),
                            delta.value.offers.len(),
                        ))
                    }
                    _ => None,
                });
            return Err(format!(
                "step={text} first_reply={disposition:?} missing_conversation_delta shop={shop:?}"
            )
            .into());
        }
    };
    if previous.is_some_and(|new| new != delta.base_revision) {
        return Err(format!(
            "step={text} conversation revision gap at {}",
            delta.base_revision
        )
        .into());
    }
    *previous = Some(delta.new_revision);
    let conversation = wire::decode_npc_conversation(&delta.payload)
        .map_err(|error| format!("step={text} conversation: {error:?}"))?;
    if conversation.open.map(|(actor, _)| actor) != Some(npc) || conversation.lines.is_empty() {
        return Err(format!("step={text} different conversation {conversation:?}").into());
    }
    capture(
        capture_dir,
        &format!("s10-{text}-conversation.bin"),
        &delta.payload,
    )?;
    Ok((
        format!(
            "{text}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            wire::STATE_DOMAIN_NPC_CONVERSATION,
            delta.server_sequence,
            delta.base_revision,
            delta.new_revision,
            identity(npc),
            conversation.lines.join("\u{1f}")
        ),
        reply.server_sequence,
    ))
}

const ANKRAHMUN_LANDING: ActorPosition = ActorPosition {
    x: 33092,
    y: 32883,
    floor: 6,
};

async fn seed_ankrahmun(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_character(clients, 0x42, 0).await?;
    character_prep::grant_item(&client, 3035, 1).await?;
    request_fixture_control(
        uuid_text(client.join_snapshot().game_session_id.as_bytes()),
        "--profile",
        "s10-rashid".into(),
    )
    .await
    .map_err(|error| format!("S10 client fixture: quest preparation refused: {error}"))?;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("S10 client fixture: initial logout refused".into());
    }
    drop(client);
    wait_character_released(clients, 0).await?;
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let mut transaction = sqlx::Connection::begin(&mut connection).await?;
    let character = clients.characters[0].as_slice();
    // The disposable account has a saved position with the live bundle's digests.
    // Seed before the next admission so its normal position and quest loaders run.
    let position = sqlx::query(
        "UPDATE game_character_last_positions SET tile_x=$2, tile_y=$3, tile_floor=$4,
         write_sequence=write_sequence+1
         WHERE character_id=encode($1,'hex')::uuid AND house_key IS NULL",
    )
    .bind(character)
    .bind(ANKRAHMUN_LANDING.x)
    .bind(ANKRAHMUN_LANDING.y)
    .bind(ANKRAHMUN_LANDING.floor)
    .execute(&mut *transaction)
    .await?;
    if position.rows_affected() != 1 {
        return Err("S10 client fixture: expected one saved position".into());
    }
    transaction.commit().await?;
    evidence("S10_CLIENT_FIXTURE position_rows=1 quest_receipt=committed");
    Ok(())
}

async fn journey(clients: &SeamClients<'_>, capture_dir: &Path) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    character_prep::declare_seeded(
        "S10.client",
        "Ankrahmun landing, Travelling Trader stage 13, one platinum coin",
        "capture Rashid's weekday shop conversation",
    );
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_Q_S10_ACCOUNT").await?;
    let clients = dedicated.clients(clients);
    seed_ankrahmun(&clients).await?;
    let mut client = connect_character(&clients, 0x42, 0).await?;
    if client.world_spatial().actor_position != ANKRAHMUN_LANDING {
        return Err(format!(
            "S10 client capture: wrong Ankrahmun admission {:?}",
            client.world_spatial().actor_position
        )
        .into());
    }
    let platinum = provisioning::item_reference(&provisioning::item_index()?, 3035)?;
    if !client.inventory().is_some_and(|inventory| {
        inventory
            .entries
            .iter()
            .any(|entry| entry.item_definition_ref == platinum && entry.count.get() == 1)
    }) {
        return Err("S10 client fixture: one platinum coin missing on re-admission".into());
    }
    let now = crate::world_clock::now_utc_ms().ok_or("S10 client capture: World clock unavailable")?;
    let destination =
        oteryn_gameplay_entities::rashid_schedule::destination_at(i64::try_from(now)?);
    if destination.city != "Thursday" {
        return Err(format!(
            "S10 client capture: Ankrahmun requires Thursday, observed {}",
            destination.city
        )
        .into());
    }
    let rashid = metadata
        .npcs
        .npcs
        .iter()
        .find(|npc| npc.key == "oteryn:npc.rashid")
        .ok_or("S10 client capture: today's Rashid placement absent")?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        rashid.placements.iter().any(|place| {
            place.x == destination.placement.x
                && place.y == destination.placement.y
                && place.floor == destination.placement.floor
                && place.floor == at.floor
                && at.x.abs_diff(place.x).max(at.y.abs_diff(place.y)) <= 3
        })
    })
    .ok_or("S10 client capture: Rashid unreachable")?;
    super::full_row_s10::walk_exact(&mut client, world, &path).await?;
    let actor = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entity| {
                entity.kind == EntityKind::Npc
                    && entity
                        .display_name
                        .as_ref()
                        .and_then(|name| name.as_str().ok())
                        == Some("Rashid")
                    && entity
                        .position
                        .x
                        .abs_diff(i32::from(destination.placement.x))
                        .max(
                            entity
                                .position
                                .y
                                .abs_diff(i32::from(destination.placement.y)),
                        )
                        <= 3
                    && entity.position.floor == -i16::from(destination.placement.floor)
            })
        })
        .map(|entity| entity.entity)
        .ok_or("S10 client capture: Rashid not visible")?;
    let result: TestResult<String> = async {
        let session = client.session_mut();
        let details = session
            .is_selected(oteryn_protocol_oteryn::trade_details::CAPABILITY_TRADE_DETAILS_V1);
        let conversation_snapshot = session
            .gated_snapshot(wire::STATE_DOMAIN_NPC_CONVERSATION)
            .ok_or("S10 client capture: missing conversation pre-state")?;
        let conversation = wire::decode_npc_conversation(&conversation_snapshot.payload)
            .map_err(|error| format!("S10 client capture: invalid conversation pre-state: {error:?}"))?;
        let window_snapshot = session
            .gated_snapshot(wire::STATE_DOMAIN_NPC_TRADE_WINDOW)
            .ok_or("S10 client capture: missing shop pre-state")?;
        let window = wire::decode_npc_trade_window_for(&window_snapshot.payload, details)
            .map_err(|error| format!("S10 client capture: invalid shop pre-state: {error:?}"))?;
        if conversation.open.is_some() || window.open.is_some() || !window.offers.is_empty() {
            return Err("S10 client capture: conversation and shop must start closed".into());
        }
        let mut previous = Some(conversation_snapshot.revision);
        let expected_window_base = window_snapshot.revision;
        let (mut expected, _) = talk(&mut client, actor, "hi", &mut previous, capture_dir).await?;
        let (line, reply_sequence) =
            talk(&mut client, actor, "trade", &mut previous, capture_dir).await?;
        expected.push_str(&line);
        let window_delta = loop {
            let delta = client
                .session_mut()
                .read_gated_delta(wire::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
                .await?;
            if delta.server_sequence > reply_sequence {
                break delta;
            }
        };
        if window_delta.base_revision != expected_window_base {
            return Err(format!("step=trade window: expected base revision {expected_window_base}, observed {}", window_delta.base_revision).into());
        }
        let window = wire::decode_npc_trade_window_for(&window_delta.payload, details)
            .map_err(|error| format!("step=trade window: {error:?}"))?;
        let revision = match window.open {
            Some((opened, revision)) if opened == actor => revision,
            other => return Err(format!("step=trade window opened {other:?}").into()),
        };
        if window.offers.is_empty()
            || window
                .offers
                .iter()
                .any(|offer| offer.count == 0 || offer.unit_price == 0)
        {
            return Err(format!("step=trade window offers {:?}", window.offers).into());
        }
        let mut lines = vec![format!("revision={revision}")];
        for offer in &window.offers {
            lines.push(format!(
                "{:?} {} x{} @{}",
                offer.side, offer.item_definition_ref, offer.count, offer.unit_price
            ));
        }
        capture(capture_dir, "s10-trade-window.bin", &window_delta.payload)?;
        expected.push_str(&format!(
            "trade\t{}\t{}\t{}\t{}\t{}\t{}\n",
            wire::STATE_DOMAIN_NPC_TRADE_WINDOW,
            window_delta.server_sequence,
            window_delta.base_revision,
            window_delta.new_revision,
            identity(actor),
            lines.join("\u{1f}")
        ));
        let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
        capture(
            capture_dir,
            "s10-session.txt",
            format!("session={session} trade_details={details}\n").as_bytes(),
        )?;
        capture(capture_dir, "s10-expected.txt", expected.as_bytes())?;
        Ok(format!("offers={}", window.offers.len()))
    }
    .await;
    let _ = client.session_mut().logout().await;
    result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let Some(capture_dir) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        evidence("e2e_all scenario=S10.client status=FAIL evidence=capture_directory_missing");
        return;
    };
    match verdict::indexed_timeout(
        Duration::from_secs(120),
        verdict::observe(journey(clients, Path::new(&capture_dir))),
    )
    .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=S10.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=S10.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=S10.client status=FAIL evidence=timeout"),
    }
}

#[test]
fn capture_refuses_existing_files_and_missing_directories() -> TestResult {
    let root = std::env::temp_dir().join(format!("oteryn-s10-capture-{}", std::process::id()));
    std::fs::create_dir(&root)?;
    let result = (|| -> TestResult {
        capture(&root, "frame.bin", b"first")?;
        assert!(capture(&root, "frame.bin", b"second").is_err());
        assert_eq!(std::fs::read(root.join("frame.bin"))?, b"first");
        assert!(capture(&root.join("missing"), "frame.bin", b"dropped").is_err());
        Ok(())
    })();
    std::fs::remove_dir_all(&root)?;
    result
}
