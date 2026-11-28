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
//! 4. Walk to today's Rashid placement, say `hi` and `trade`, and assert that the native
//!    conversation opens a shop, and assert the compiled native actor has exactly the one
//!    placement selected by the node's server-save weekday rotation (weekday shop gate).
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

// Paid arrivals from content/services/travel/travel-00000-00056.json, in the runtime frame.
const TRAVEL_NPCS: &[(&str, &str, &str, u64, ActorPosition)] = &[
    (
        "oteryn:npc.captain_bluebear",
        "port hope",
        "Port Hope",
        160,
        ActorPosition {
            x: 32527,
            y: 32784,
            floor: 6,
        },
    ),
    (
        "oteryn:npc.captain_bluebear",
        "edron",
        "Edron",
        160,
        ActorPosition {
            x: 33175,
            y: 31764,
            floor: 6,
        },
    ),
    (
        "oteryn:npc.captain_seagull",
        "thais",
        "Thais",
        130,
        ActorPosition {
            x: 32310,
            y: 32210,
            floor: 6,
        },
    ),
    (
        "oteryn:npc.kendra",
        "thais",
        "Thais",
        180,
        ActorPosition {
            x: 32259,
            y: 32262,
            floor: 7,
        },
    ),
];

const ACTIONS: &[&str] = &[
    "quote",
    "refusal",
    "return-to-captain",
    "travel",
    "rashid-gate",
    "fresh-admission",
];

fn gap(attempted: &[&str]) -> Option<&'static str> {
    ACTIONS.get(attempted.len()).copied()
}

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
        let status = if result.is_ok() && self.attempted == ACTIONS {
            "PASS"
        } else {
            "FAIL"
        };
        if result.is_ok() && status == "FAIL" {
            evidence(&format!(
                "S10_GAP status=FAIL step={} observed=missing-required-observation",
                gap(&self.attempted).unwrap_or("journey")
            ));
        }
        if let Err(error) = &result {
            evidence(&format!(
                "S10_GAP status=FAIL step={} attempted={:?} observed={error}",
                gap(&self.attempted).unwrap_or("cleanup"),
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
                    let distance = entry
                        .position
                        .x
                        .abs_diff(i32::from(p.x))
                        .max(entry.position.y.abs_diff(i32::from(p.y)));
                    entry.position.floor == -i16::from(p.floor)
                        && if npc.key == "oteryn:npc.rashid" {
                            distance <= 3
                                && entry
                                    .display_name
                                    .as_ref()
                                    .and_then(|name| name.as_str().ok())
                                    == Some("Rashid")
                        } else {
                            distance == 0
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

async fn approach_rashid(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult<oteryn_dev_client::EntityRef> {
    let today = utc_rashid_destination()?;
    let expected = rashid_bundle_cell(today);
    let mut npc = npcs
        .npcs
        .iter()
        .find(|npc| npc.key == "oteryn:npc.rashid")
        .cloned()
        .ok_or("step=rashid-gate need=authored-Rashid-placement")?;
    npc.placements.retain(|p| (p.x, p.y, p.floor) == expected);
    if npc.placements.len() != 1 {
        return Err(format!(
            "step=rashid-gate need=unique-{}-placement-{expected:?}",
            today.city
        )
        .into());
    }
    let (x, y, floor) = expected;
    let from = tile(client.world_spatial().actor_position)?;
    // Search the connected tiles of the bounded compiled bundle for inter-town travel.
    let path = find_route(
        from,
        |at| at.floor == floor && at.x.abs_diff(x).max(at.y.abs_diff(y)) <= 3,
        |at, direction| destination(world, at, direction),
    )
    .map_err(|_| {
        format!(
            "step=rashid-gate need=connected-walking-route-to-{}-Rashid-{expected:?}",
            today.city
        )
    })?;
    // Keep a blocked step's detour goal inside the local route window.
    for segment in path.chunks(64) {
        walk(client, world, segment).await?;
    }
    for _ in 0..8 {
        if utc_rashid_destination()? != today {
            return Err("step=rashid-gate observed=weekday-changed-on-route".into());
        }
        let actor = actor_at(client, &npc)
            .ok_or("step=rashid-gate need=live-Rashid-at-weekday-placement")?;
        let visible = client
            .world_entities()
            .ok_or("step=rashid-gate need=live-entities")?;
        if visible
            .iter()
            .filter(|entry| {
                entry.kind == EntityKind::Npc
                    && entry
                        .display_name
                        .as_ref()
                        .and_then(|name| name.as_str().ok())
                        == Some("Rashid")
            })
            .count()
            != 1
        {
            return Err("step=rashid-gate need=exactly-one-live-visible-Rashid".into());
        }
        let live = tile(
            visible
                .get(&actor)
                .ok_or("step=rashid-gate need=live-Rashid-position")?
                .position,
        )?;
        let here = tile(client.world_spatial().actor_position)?;
        if in_talk_range(here, live) {
            return Ok(actor);
        }
        let path = route(world, here, |at| in_talk_range(at, live))
            .ok_or("step=rashid-gate need=route-to-live-Rashid-talk-range")?;
        walk(client, world, &path).await?;
    }
    Err("step=rashid-gate need=Rashid-to-remain-in-talk-range".into())
}

fn in_talk_range(player: TilePos, npc: TilePos) -> bool {
    player.floor == npc.floor
        && u32::from(player.x.abs_diff(npc.x).max(player.y.abs_diff(npc.y)))
            <= oteryn_protocol_oteryn::npc_service::TALK_RANGE_TILES
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

fn rashid_bundle_cell(day: RashidDestination) -> (u16, u16, i8) {
    (day.placement.x, day.placement.y, day.placement.floor)
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
    let expected_rashid = utc_rashid_destination()?;
    let (key, keyword, display, fare, arrival, npc) = TRAVEL_NPCS
        .iter()
        .filter(|&&(_, keyword, _, _, _)| {
            keyword != "port hope" || expected_rashid.city == "Wednesday"
        })
        .find_map(|&(key, keyword, display, fare, arrival)| {
            metadata.npcs.npcs.iter().find(|npc| npc.key == key)
                .filter(|npc| route(world, here, |at| npc.placements.iter().any(|p| {
                    p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3
                })).is_some())
                .map(|npc| (key, keyword, display, fare, arrival, npc))
        })
        .ok_or_else(|| {
            format!(
                "step=travel-plan need=reachable-paid-departure for {} Rashid",
                expected_rashid.city
            )
        })?;
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
    if utc_rashid_destination()? != expected_rashid {
        return Err(
            "step=travel-plan observed=weekday-changed-during-fare-hunt need=new-Rashid-itinerary"
                .into(),
        );
    }
    let funded_gold = carried_gold(&client)?;
    let actor = approach(&mut client, world, npc).await?;
    let departure = client.world_spatial().actor_position;
    row.step("return-to-captain", (key, departure, funded_gold));
    let _ = talk(&mut client, actor, "hi").await?;
    let _ = talk(&mut client, actor, keyword).await?;
    let (travelled, inventory_changed, _) = talk(&mut client, actor, "yes").await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    let mut inventory_changed = inventory_changed;
    while tokio::time::Instant::now() < deadline
        && (!inventory_changed || client.world_spatial().actor_position != arrival)
    {
        client.service_liveness(Duration::from_millis(25)).await?;
        inventory_changed |= client
            .take_events()
            .iter()
            .any(|event| matches!(event, SessionEvent::Inventory(_)));
    }
    if !travelled.iter().any(|line| line.contains("Set the sails"))
        || !inventory_changed
        || client.world_spatial().actor_position != arrival
        || carried_gold(&client)? + fare != funded_gold
    {
        return Err(format!("step travel: observed reply={travelled:?} position={:?} carried_gold={}; need authored arrival {arrival:?} and atomic fare burn", client.world_spatial().actor_position, carried_gold(&client)?).into());
    }
    row.step("travel", &travelled);
    let rashid_actor = approach_rashid(&mut client, world, &metadata.npcs).await?;
    let _ = talk(&mut client, rashid_actor, "hi").await?;
    let (trade, _, shop_opened) = talk(&mut client, rashid_actor, "trade").await?;
    if trade.is_empty() || !shop_opened {
        return Err("step rashid-gate: trade command returned no conversation/shop delta; need weekday destination shop availability".into());
    }
    row.step(
        "rashid-gate",
        (
            expected_rashid.city,
            rashid_bundle_cell(utc_rashid_destination()?),
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
    let fresh_rashid = approach_rashid(&mut fresh, world, &metadata.npcs).await?;
    let _ = talk(&mut fresh, fresh_rashid, "hi").await?;
    let (fresh_trade, _, fresh_shop) = talk(&mut fresh, fresh_rashid, "trade").await?;
    if fresh_trade.is_empty() || !fresh_shop {
        return Err(format!("step fresh-admission: Rashid shop was not restored; observed reply={fresh_trade:?} shop_delta={fresh_shop}").into());
    }
    row.step("fresh-admission", (saved_position, saved_gold, fresh_trade));
    if fresh.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=final-logout need=accepted-logout-after-rashid-shop".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weekday_schedule_matches_the_bundle_npc_floor() {
        let sunday =
            oteryn_gameplay_entities::rashid_schedule::destination_at(3 * 86_400_000 + 9 * 3_600_000);
        assert_eq!(rashid_bundle_cell(sunday), (32328, 31782, -6));
    }

    #[test]
    fn funded_travel_requires_return_to_the_npc() {
        assert_eq!(gap(&["quote", "refusal"]), Some("return-to-captain"));
        assert_eq!(gap(ACTIONS), None);
    }
}

#[test]
fn npc_wander_requires_range_to_the_live_actor() {
    let player = TilePos {
        x: 100,
        y: 100,
        floor: -7,
    };
    let authored = TilePos { x: 103, ..player };
    let wandered = TilePos { x: 106, ..player };
    assert!(in_talk_range(player, authored));
    assert!(!in_talk_range(player, wandered));
    assert!(in_talk_range(TilePos { x: 102, ..player }, wandered));
    assert!(!in_talk_range(
        TilePos {
            floor: -6,
            ..wandered
        },
        wandered
    ));
}
