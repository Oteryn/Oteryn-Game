//! S10 player scenario, derived from the S10 matrix/audit row and its native owners
//! (`paid_travel.rs`, `npc_actor.rs`, `npc_voice.rs`, and `npc_trade_command.rs`):
//!
//! 1. Walk from the admitted position to an authored paid-travel NPC; say `hi`, send a
//!    destination keyword, and assert the accepted command replies and quoted fare.
//! 2. Send `yes` without enough carried money and assert the NPC's insufficient-funds
//!    refusal, with no relocation or inventory delta.
//! 3. Obtain the fare only by walking to authored rat spawns, fighting, and looting; repeat
//!    `hi`, destination, `yes`, then assert “Set the sails!”, an inventory delta, and the
//!    authoritative spatial relocation to the authored arrival.
//! 4. Walk to today's UTC Rashid placement, say `hi` and `trade`, and assert that the native
//!    conversation opens a shop, and assert the compiled native actor has exactly the one
//!    placement selected by the node's UTC weekday rotation (weekday shop gate).
//! 5. Log out, make a fresh admission, and re-check the paid arrival and spent carried money;
//!    walk back to today's Rashid and re-check that the shop remains available.
//!
//! Every transition below is a native client command. No database or privileged fixture
//! mutation is used; a missing authored route is reported only after the admitted player has
//! attempted the first NPC speech command that can be reached.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, SessionEvent};
use oteryn_gameplay_entities::rashid_schedule::RashidDestination;
use oteryn_protocol_oteryn::npc_service as wire;

const TRAVEL_NPCS: &[(&str, &str, &str, u64)] = &[
    ("oteryn:npc.captain_bluebear", "edron", "Edron", 160),
    ("oteryn:npc.captain_seagull", "thais", "Thais", 130),
    ("oteryn:npc.kendra", "thais", "Thais", 180),
];

struct Row {
    started: std::time::Instant,
    attempted: Vec<&'static str>,
}

impl Row {
    fn new() -> Self {
        Self {
            started: std::time::Instant::now(),
            attempted: Vec::new(),
        }
    }
    fn step(&mut self, name: &'static str, observed: impl std::fmt::Debug) {
        self.attempted.push(name);
        evidence(&format!(
            "S10_CHECK step={name} status=PASS observed={observed:?}"
        ));
    }
    fn finish(self, result: TestResult) {
        let status = if result.is_ok()
            && self.attempted
                == [
                    "quote",
                    "refusal",
                    "travel",
                    "rashid-gate",
                    "fresh-admission",
                ] {
            "PASS"
        } else {
            "FAIL"
        };
        if let Err(error) = &result {
            evidence(&format!(
                "S10_CHECK status=FAIL attempted={:?} observed={error}",
                self.attempted
            ));
        }
        evidence(&format!(
            "e2e_all scenario=S10.full status={status} seconds={:.3}",
            self.started.elapsed().as_secs_f64()
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = verdict::indexed_timeout(Duration::from_secs(360), verdict::observe(journey(&mut row, clients))).await;
    row.finish(match result { Ok(result) => result, Err(_) => Err("step deadline: no server reply within 360s; need the server to complete the pending NPC/travel command".into()) });
}

async fn talk(
    client: &mut DevClientSession,
    npc: oteryn_dev_client::EntityRef,
    text: &str,
) -> TestResult<(Vec<String>, bool, bool)> {
    client.take_events();
    let payload = wire::encode_npc_talk_intent(&wire::NpcTalkIntent {
        npc_actor: npc,
        text: text.into(),
    })
    .map_err(|error| format!("step talk-{text}: encoder refused: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_NPC_SERVICE_V1,
            wire::COMMAND_TYPE_NPC_TALK_INTENT,
            &payload,
        )
        .await?;
    let disposition = wire::decode_npc_intent_result(&reply.payload)
        .map_err(|error| format!("step talk-{text}: reply decode: {error:?}"))?;
    if reply.status != CommandStatus::Accepted || disposition != wire::NpcIntentDisposition::Ok {
        return Err(format!("step talk-{text}: observed status={:?} disposition={disposition:?}; need an accepted native NPC reply", reply.status).into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    let mut inventory_changed = false;
    let mut shop_opened = false;
    let mut lines = None;
    loop {
        for event in client.take_events() {
            match event {
                SessionEvent::NpcConversation(delta)
                    if delta.server_sequence > reply.server_sequence =>
                {
                    lines = Some(delta.value.lines);
                }
                SessionEvent::Inventory(_) => inventory_changed = true,
                SessionEvent::NpcTradeWindow(delta)
                    if delta.server_sequence > reply.server_sequence =>
                {
                    shop_opened = true
                }
                _ => {}
            }
        }
        if (text != "trade" || shop_opened) && lines.is_some() {
            return Ok((
                lines.take().unwrap_or_default(),
                inventory_changed,
                shop_opened,
            ));
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step talk-{text}: accepted at sequence {} but conversation delta timed out; need STATE_DOMAIN_NPC_CONVERSATION", reply.server_sequence).into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
}

fn actor_at(
    client: &DevClientSession,
    npc: &oteryn_world_bundle::npc::Npc,
) -> Option<oteryn_dev_client::EntityRef> {
    client
        .world_entities()?
        .iter()
        .find(|entry| {
            entry.kind == EntityKind::Npc
                && npc.placements.iter().any(|p| {
                    entry.position
                        == ActorPosition {
                            x: i32::from(p.x),
                            y: i32::from(p.y),
                            floor: -i16::from(p.floor),
                        }
                })
        })
        .map(|entry| entry.entity)
}

async fn approach(
    client: &mut DevClientSession,
    world: &crate::map::boot::BundleWorld,
    npc: &oteryn_world_bundle::npc::Npc,
) -> TestResult<oteryn_dev_client::EntityRef> {
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| npc.placements.iter().any(|p| p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3))
        .ok_or_else(|| format!("step approach-{}: no legitimate walking route inside the compiled town window; need authored reachable placement/travel chain", npc.key))?;
    walk(client, world, &path).await?;
    actor_at(client, npc).ok_or_else(|| {
        format!(
            "step approach-{}: walked to authored placement but no NPC entity delta was served",
            npc.key
        )
        .into()
    })
}

fn carried_gold(client: &DevClientSession) -> TestResult<u64> {
    let index = provisioning::item_index()?;
    let coins = [
        (provisioning::item_reference(&index, 3031)?, 1_u64),
        (provisioning::item_reference(&index, 3035)?, 100),
        (provisioning::item_reference(&index, 3043)?, 10_000),
    ];
    Ok(client
        .inventory()
        .ok_or("step inventory: inventory state absent")?
        .entries
        .iter()
        .map(|entry| {
            coins
                .iter()
                .find(|(key, _)| key == &entry.item_definition_ref)
                .map_or(0, |(_, value)| u64::from(entry.count.get()) * value)
        })
        .sum())
}

fn utc_rashid_destination() -> TestResult<RashidDestination> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| format!("step rashid-gate: server clock before epoch: {error}"))?;
    Ok(oteryn_gameplay_entities::rashid_schedule::destination_at(
        i64::try_from(elapsed.as_millis())?,
    ))
}

async fn earn(
    client: &mut DevClientSession,
    world: &crate::map::boot::BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    needed: u64,
) -> TestResult {
    let rats: Vec<_> = spawns
        .sources
        .iter()
        .flat_map(|source| {
            source.points.iter().filter_map(|point| {
                (spawns.creatures[point.creature as usize].key == "oteryn:creature.rat")
                    .then_some((point.x, point.y, source.floor))
            })
        })
        .collect();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    while carried_gold(client)? < needed {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step earn-fare: combat/loot budget expired; observed carried_gold={}, required={needed}; need sufficient authored player-obtainable loot", carried_gold(client)?).into());
        }
        let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
            rats.iter().any(|&(x, y, floor)| {
                at.x.abs_diff(x).max(at.y.abs_diff(y)) <= 1 && at.floor == floor
            })
        })
        .ok_or("step earn-fare: no legitimate route to an authored rat spawn")?;
        walk(client, world, &path).await?;
        client_hunt::hunt(client)
            .await
            .map_err(|error| { verdict::record_error(verdict::IndexedError::as_error(&error)); verdict::context(format!("step earn-fare: real combat/loot failed: {error}"), error) })?;
    }
    Ok(())
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_character(clients, 0xf8, 1).await?;
    let here = tile(client.world_spatial().actor_position)?;
    let (key, keyword, display, fare, npc) = TRAVEL_NPCS
        .iter()
        .find_map(|&(key, keyword, display, fare)| {
            metadata
                .npcs
                .npcs
                .iter()
                .find(|npc| npc.key == key)
                .filter(|npc| {
                    route(world, here, |at| {
                        npc.placements.iter().any(|p| {
                            p.floor == at.floor
                                && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3
                        })
                    })
                    .is_some()
                })
                .map(|npc| (key, keyword, display, fare, npc))
        })
        .ok_or("step quote: no authored S10 paid-travel NPC is reachable by legitimate walking from admission; need a compiled route to a travel service")?;
    let actor = approach(&mut client, world, npc).await?;
    let _ = talk(&mut client, actor, "hi").await?;
    let (quote, _, _) = talk(&mut client, actor, keyword).await?;
    if !quote
        .iter()
        .any(|line| line.contains(display) && line.contains(&fare.to_string()))
    {
        return Err(format!(
            "step quote: {key} observed reply={quote:?}; need destination and fare {fare}"
        )
        .into());
    }
    row.step("quote", &quote);
    let before_position = client.world_spatial().actor_position;
    let before_gold = carried_gold(&client)?;
    let (refusal, inventory, _) = talk(&mut client, actor, "yes").await?;
    if before_gold < fare
        && (!refusal.iter().any(|line| line.contains("enough money"))
            || inventory
            || client.world_spatial().actor_position != before_position)
    {
        return Err(format!("step refusal: observed reply={refusal:?} inventory_delta={inventory} position={:?}; need insufficient-funds refusal without mutation", client.world_spatial().actor_position).into());
    }
    if before_gold >= fare {
        return Err(format!("step refusal: confirmation unexpectedly had enough money ({before_gold} >= {fare}) and observed reply={refusal:?}; need an ordinary unfunded character for the negative path").into());
    }
    row.step("refusal", &refusal);
    earn(&mut client, world, &metadata.spawns, fare).await?;
    let funded_gold = carried_gold(&client)?;
    let _ = talk(&mut client, actor, "hi").await?;
    let _ = talk(&mut client, actor, keyword).await?;
    let (travelled, inventory_changed, _) = talk(&mut client, actor, "yes").await?;
    if !travelled.iter().any(|line| line.contains("Set the sails"))
        || !inventory_changed
        || client.world_spatial().actor_position == before_position
        || carried_gold(&client)? + fare != funded_gold
    {
        return Err(format!("step travel: observed reply={travelled:?} position={:?} carried_gold={}; need relocation and atomic fare burn", client.world_spatial().actor_position, carried_gold(&client)?).into());
    }
    row.step("travel", &travelled);
    let rashid = metadata.npcs.npcs.iter().find(|npc| npc.key == "oteryn:npc.rashid").ok_or("step rashid-gate: compiled world omitted today's Rashid placement; need the node boot weekday rotation to retain it")?;
    let expected_rashid = utc_rashid_destination()?;
    let observed_rashid = rashid
        .placements
        .iter()
        .map(|placement| (placement.x, placement.y, placement.floor))
        .collect::<Vec<_>>();
    let expected_placement = expected_rashid.placement;
    let expected_rashid_position = (
        expected_placement.x,
        expected_placement.y,
        expected_placement.floor,
    );
    if observed_rashid.as_slice() != [expected_rashid_position] {
        return Err(format!("step rashid-gate: observed compiled placements={observed_rashid:?}; need UTC {} placement={expected_rashid_position:?}", expected_rashid.city).into());
    }
    let rashid_actor = approach(&mut client, world, rashid).await?;
    let _ = talk(&mut client, rashid_actor, "hi").await?;
    let (trade, _, shop_opened) = talk(&mut client, rashid_actor, "trade").await?;
    if trade.is_empty() || !shop_opened {
        return Err("step rashid-gate: trade command returned no conversation/shop delta; need weekday destination shop availability".into());
    }
    row.step(
        "rashid-gate",
        (
            expected_rashid.city,
            expected_rashid_position,
            client.world_spatial().actor_position,
            &trade,
        ),
    );
    let saved_position = client.world_spatial().actor_position;
    let saved_gold = carried_gold(&client)?;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step fresh-admission: logout refused".into());
    }
    drop(client);
    let mut fresh = connect_character(clients, 0xf9, 1).await?;
    if fresh.world_spatial().actor_position != saved_position || carried_gold(&fresh)? != saved_gold
    {
        return Err(format!("step fresh-admission: durable state mismatch position={:?}/{saved_position:?} gold={}/{}", fresh.world_spatial().actor_position, carried_gold(&fresh)?, saved_gold).into());
    }
    let fresh_rashid = approach(&mut fresh, world, rashid).await?;
    let _ = talk(&mut fresh, fresh_rashid, "hi").await?;
    let (fresh_trade, _, fresh_shop) = talk(&mut fresh, fresh_rashid, "trade").await?;
    if fresh_trade.is_empty() || !fresh_shop {
        return Err(format!("step fresh-admission: Rashid shop was not restored; observed reply={fresh_trade:?} shop_delta={fresh_shop}").into());
    }
    row.step("fresh-admission", (saved_position, saved_gold, fresh_trade));
    let _ = fresh.session_mut().logout().await?;
    Ok(())
}
