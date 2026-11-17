//! MP05: two real players trade with the same authored NPC independently.
//!
//! Reference: Crystal 201189f871d23597b82caf88fd611454a2ddefb8,
//! data/npclib/npc_system/npc_handler.lua:203-224 (per-player interaction),
//! data-global/npc/sam.lua:162-246,249-254 (club 5/1, throwing knife 25/2),
//! src/creatures/npcs/npc.cpp:409-413 (insufficient gold),
//! src/lua/functions/creatures/npc/npc_functions.cpp:648-654 (charge purchased units).
//! All quantities, inventory occupancy and balances below come from live players;
//! no negative test treats an unrelated refusal as proof of its prerequisite.
use super::*;
use bots::Bot;
use oteryn_dev_client::{DevClientSession, EntityKind, EntityRef};
use oteryn_protocol_oteryn::{CommandStatus, npc_service as wire};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
struct Checks {
    errors: Vec<String>,
    seen: BTreeSet<String>,
}

fn check(case: &str, result: TestResult, failures: &mut Checks) -> bool {
    failures.seen.insert(case.to_owned());
    match result {
        Ok(()) => {
            evidence(&format!("MP_CHECK MP05 {case} PASS verified"));
            true
        }
        Err(error) => {
            verdict::record_assertion(case);
            evidence(&format!(
                "MP_CHECK MP05 {case} FAIL {error}; source:{}:{}",
                file!(),
                line!()
            ));
            failures.errors.push(format!("{case}: {error}"));
            false
        }
    }
}

async fn bounded<T>(
    phase: &'static str,
    seconds: u64,
    action: impl std::future::Future<Output = TestResult<T>>,
) -> TestResult<T> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(seconds);
    verdict::Harness::global().bounded_wait(deadline, phase);
    tokio::time::timeout_at(deadline, action)
        .await
        .map_err(|_| format!("MP05 {phase} deadline; no server reply before deadline"))?
}

pub(super) async fn run(
    core: &SeamClients<'_>,
    packets: &SeamClients<'_>,
    world: &BundleWorld,
    _towns: &oteryn_world_bundle::subarea::Table,
) -> TestResult {
    let started = std::time::Instant::now();
    let mut failures = Checks::default();
    // Retain partially admitted sockets for cleanup; connect_bots bounds each admission.
    let (mut players, admission) = bots::connect_bots(&[(core, 0, None), (packets, 0, None)]).await;
    let ready = check("admissions", admission, &mut failures);
    let journey = if ready {
        bounded(
            "npc-trade",
            240,
            journey(&mut players, world, &mut failures),
        )
        .await
    } else {
        Err(format!("dependent on admissions; {}", failures.errors.join("; ")).into())
    };
    check("journey", journey, &mut failures);
    // Always attempt relog, including after a rejected trade.
    let relog = if ready {
        bounded("relog", 120, relog(&mut players)).await
    } else {
        Err(format!("dependent on admissions; {}", failures.errors.join("; ")).into())
    };
    check("relog", relog, &mut failures);
    check(
        "release",
        bounded(
            "release",
            1800,
            bots::release_all(&mut players, core, packets),
        )
        .await,
        &mut failures,
    );
    // Early refusals and deadlines fail every dependent row with their original cause.
    let cause = failures.errors.join("; ");
    for case in [
        "same-vendor",
        "conversation-isolation",
        "independent-windows",
        "stale-price-0",
        "stale-price-1",
        "buy-sell-0",
        "buy-sell-1",
        "greet",
        "trade-window",
        "unsupported-sale",
        "insufficient-gold",
        "full-backpack",
        "concurrent-buy",
        "bye",
    ] {
        if !failures.seen.contains(case) {
            check(
                case,
                Err(format!("dependent check blocked: {cause}").into()),
                &mut failures,
            );
        }
    }
    let status = if failures.errors.is_empty() {
        "PASS"
    } else {
        "FAIL"
    };
    evidence(&format!(
        "e2e_all scenario=MP05.full status={status} seconds={:.3}",
        started.elapsed().as_secs_f64()
    ));
    if failures.errors.is_empty() {
        Ok(())
    } else {
        Err(failures.errors.join("; ").into())
    }
}

async fn vendor(players: &mut [Bot<'_, '_>], world: &BundleWorld) -> TestResult<Vec<EntityRef>> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let sam = metadata
        .npcs
        .npcs
        .iter()
        .find(|npc| npc.key == "oteryn:npc.sam")
        .ok_or("authored Sam absent; source:content/npcs/definitions/index.json:1")?;
    let mut actors = Vec::new();
    let mut reserved = BTreeSet::new();
    for index in 0..players.len() {
        let client = players[index].client()?;
        let here = tile(client.world_spatial().actor_position)?;
        let path = route(world, here, |at| {
            sam.placements.iter().any(|p| {
                !reserved.contains(&(at.floor, at.x, at.y))
                    && p.floor == at.floor
                    && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3
                    && (at.x != p.x || at.y != p.y)
            })
        })
        .ok_or_else(|| format!("bot={index} no player-walk route to Sam from {here:?}"))?;
        let (first, rest) = players.split_at_mut(1);
        let (walker, witness) = if index == 0 {
            (&mut first[0], &mut rest[0])
        } else {
            (&mut rest[0], &mut first[0])
        };
        let finished = std::sync::atomic::AtomicBool::new(false);
        let (walked, serviced) = tokio::join!(
            async {
                let result = async { walk(walker.client()?, world, &path).await }.await;
                finished.store(true, std::sync::atomic::Ordering::Relaxed);
                result
            },
            async {
                while !finished.load(std::sync::atomic::Ordering::Relaxed) {
                    witness
                        .client()?
                        .service_liveness(Duration::from_millis(20))
                        .await?;
                }
                TestResult::Ok(())
            }
        );
        walked?;
        serviced?;
        let client = players[index].client()?;
        let at = tile(client.world_spatial().actor_position)?;
        reserved.insert((at.floor, at.x, at.y));
        let actor = client
            .world_entities()
            .and_then(|entities| {
                entities.iter().find(|entry| {
                    entry.kind == EntityKind::Npc
                        && sam.placements.iter().any(|p| {
                            entry.position.x == i32::from(p.x)
                                && entry.position.y == i32::from(p.y)
                                && entry.position.floor == -i16::from(p.floor)
                        })
                })
            })
            .map(|entry| entry.entity)
            .ok_or("Sam absent from real entity snapshot")?;
        actors.push(actor);
        bots::pump(players).await?;
    }
    if actors[0] != actors[1] {
        return Err(format!("players observed different Sam actors: {actors:?}").into());
    }
    Ok(actors)
}

// Use the real gated command directly so failure evidence retains status AND payload.
async fn command(
    client: &mut DevClientSession,
    kind: u32,
    payload: &[u8],
    expected: wire::NpcIntentDisposition,
) -> TestResult<String> {
    let reply = client
        .session_mut()
        .gated_command(wire::CAPABILITY_NPC_SERVICE_V1, kind, payload)
        .await?;
    let detail = format!(
        "command={kind} status={:?} payload={:?}",
        reply.status, reply.payload
    );
    if !matches!(
        reply.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        return Err(format!("{detail}; source:crates/gameplay/dispatch/src/gameplay_transport/connection/routing/npc_shop.rs:1").into());
    }
    let disposition = wire::decode_npc_intent_result(&reply.payload)
        .map_err(|error| format!("{detail} decode={error:?}"))?;
    if disposition != expected {
        return Err(format!("{detail} disposition={disposition:?} expected={expected:?}; source:crates/gameplay/dispatch/src/gameplay_transport/npc_command.rs:395; source:crates/gameplay/dispatch/src/gameplay_transport/npc_trade_command.rs:477").into());
    }
    Ok(format!("{detail} disposition={disposition:?}"))
}

async fn talk(client: &mut DevClientSession, actor: EntityRef, text: &str) -> TestResult {
    let payload = wire::encode_npc_talk_intent(&wire::NpcTalkIntent {
        npc_actor: actor,
        text: text.into(),
    })
    .map_err(|error| format!("talk encode={error:?}"))?;
    command(
        client,
        wire::COMMAND_TYPE_NPC_TALK_INTENT,
        &payload,
        wire::NpcIntentDisposition::Ok,
    )
    .await?;
    Ok(())
}

async fn trade(
    client: &mut DevClientSession,
    intent: &wire::NpcTradeIntent,
    expected: wire::NpcIntentDisposition,
) -> TestResult<String> {
    let payload =
        wire::encode_npc_trade_intent(intent).map_err(|error| format!("trade encode={error:?}"))?;
    command(
        client,
        wire::COMMAND_TYPE_NPC_TRADE_INTENT,
        &payload,
        expected,
    )
    .await
}

async fn window(
    client: &mut DevClientSession,
    actor: EntityRef,
) -> TestResult<wire::NpcTradeWindow> {
    talk(client, actor, "trade").await?;
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let window = wire::decode_npc_trade_window(&delta.payload)
        .map_err(|error| format!("window payload={:?} decode={error:?}", delta.payload))?;
    if window.open.is_none_or(|(npc, _)| npc != actor) || window.offers.is_empty() {
        return Err(format!("trade window={window:?}").into());
    }
    Ok(window)
}

fn quote(
    window: &wire::NpcTradeWindow,
    side: wire::NpcTradeSide,
    definition: Option<u32>,
) -> TestResult<(wire::NpcTradeIntent, wire::NpcOffer)> {
    let (index, offer) = window
        .offers
        .iter()
        .enumerate()
        .filter(|(_, offer)| {
            offer.side == side
                && definition.is_none_or(|id| offer.item_definition_ref == id)
                && (side == wire::NpcTradeSide::Sell
                    || window.offers.iter().any(|sell| {
                        sell.side == wire::NpcTradeSide::Sell
                            && sell.item_definition_ref == offer.item_definition_ref
                            && sell.count == offer.count
                    }))
        })
        .min_by_key(|(_, offer)| offer.unit_price)
        .ok_or_else(|| format!("no matching buy/sell quote; window={window:?}"))?;
    let (npc_actor, catalogue_revision) = window.open.ok_or("trade window closed")?;
    Ok((
        wire::NpcTradeIntent {
            npc_actor,
            catalogue_revision,
            offer_index: u32::try_from(index)?,
            side,
            quantity: 1,
            expected_unit_price: offer.unit_price,
        },
        *offer,
    ))
}

#[derive(Debug, PartialEq, Eq)]
struct Snapshot {
    inventory: BTreeMap<u32, u64>,
    gold: u64,
}

async fn snapshot(bot: &mut Bot<'_, '_>) -> TestResult<Snapshot> {
    let client = bot.client()?;
    client.service_liveness(Duration::from_millis(20)).await?;
    use oteryn_protocol_oteryn::atelier::resources;
    let reply = client
        .session_mut()
        .gated_command(
            resources::CAPABILITY_ATELIER_RESOURCES_V1,
            resources::COMMAND_TYPE_ATELIER_RESOURCES_QUERY,
            &resources::encode_query(),
        )
        .await?;
    let balance_reply = format!(
        "balances status={:?} payload={:?}",
        reply.status, reply.payload
    );
    if !matches!(
        reply.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        return Err(balance_reply.into());
    }
    let balances = resources::decode_reply(&reply.payload)
        .map_err(|error| format!("{balance_reply} decode={error:?}"))?;
    let gold = balances
        .carried_gold
        .checked_add(balances.bank_gold)
        .ok_or("gold overflow")?;
    let mut inventory = BTreeMap::new();
    for entry in &client
        .inventory()
        .ok_or("inventory snapshot absent")?
        .entries
    {
        *inventory
            .entry(entry.item_definition_ref.get())
            .or_default() += u64::from(entry.count.get());
    }
    let mut db = sqlx::PgConnection::connect(bot.pair.url).await?;
    let durable: i64 = sqlx::query_scalar("SELECT COALESCE((SELECT sum(i.quantity * CASE i.definition_production_key WHEN 'oteryn:item.tibia.i3031' THEN 1 WHEN 'oteryn:item.tibia.i3035' THEN 100 WHEN 'oteryn:item.tibia.i3043' THEN 10000 ELSE 0 END)::bigint FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.lifecycle=1),0) + COALESCE((SELECT balance FROM game_account_bank_balances WHERE account_id=$2::uuid AND world_id=encode($3,'hex')::uuid),0)")
        .bind(bot.character.as_slice()).bind(&bot.pair.accounts[bot.index]).bind(bot.pair.world.as_slice()).fetch_one(&mut db).await?;
    let index = provisioning::item_index()?;
    // The client inventory contains the main backpack's direct entries. Compare
    // the whole map so a sold-out item cannot disappear only from the wire view.
    let rows: Vec<(String, i64)> = sqlx::query_as("SELECT i.definition_production_key,sum(i.quantity)::bigint FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) JOIN game_item_container_slots b ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id WHERE e.character_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 GROUP BY i.definition_production_key LIMIT 1025")
        .bind(bot.character.as_slice()).bind(bot.pair.world.as_slice()).fetch_all(&mut db).await?;
    let mut stored = BTreeMap::new();
    for (key, quantity) in rows {
        let revision = index
            .revision(&key)
            .ok_or("durable Item absent from pinned index")?;
        let reference = index
            .definition_ref("Item", &key, revision)
            .ok_or("durable Item reference absent")?;
        stored.insert(reference.get(), u64::try_from(quantity)?);
    }
    if stored != inventory {
        return Err(format!("wire inventory={inventory:?} durable inventory={stored:?}").into());
    }
    db.close().await?;
    if u64::try_from(durable)? != gold {
        return Err(format!("wire balances={balances:?} durable_gold={durable}").into());
    }
    Ok(Snapshot { inventory, gold })
}

async fn journey(
    players: &mut [Bot<'_, '_>],
    world: &BundleWorld,
    failures: &mut Checks,
) -> TestResult {
    let actors = vendor(players, world).await?;
    check("same-vendor", Ok(()), failures);
    talk(players[0].client()?, actors[0], "hi").await?;
    check("greet", Ok(()), failures);
    let first = window(players[0].client()?, actors[0]).await?;
    check("trade-window", Ok(()), failures);
    let (request, _) = knife_quote(&first, wire::NpcTradeSide::Buy)?;
    let before = snapshot(&mut players[1]).await?;
    let isolation = async {
        let reply = trade(
            players[1].client()?,
            &request,
            wire::NpcIntentDisposition::NoConversation,
        )
        .await?;
        let after = snapshot(&mut players[1]).await?;
        if before != after {
            return Err(format!("{reply} before={before:?} after={after:?}").into());
        }
        Ok(())
    }
    .await;
    check("conversation-isolation", isolation, failures);
    talk(players[1].client()?, actors[1], "hi").await?;
    let second = window(players[1].client()?, actors[1]).await?;
    if first != second {
        return Err(format!("independent windows differ A={first:?} B={second:?}").into());
    }
    check("independent-windows", Ok(()), failures);
    let result = unsupported_sale(&mut players[0], &first).await;
    check("unsupported-sale", result, failures);
    let result = insufficient_gold(&mut players[0], &first).await;
    check("insufficient-gold", result, failures);
    let result = concurrent_buy(players, [&first, &second]).await;
    check("concurrent-buy", result, failures);
    for (index, window) in [&first, &second].into_iter().enumerate() {
        let stale = async {
            let before = snapshot(&mut players[index]).await?;
            let (mut intent, _) = quote(window, wire::NpcTradeSide::Buy, None)?;
            intent.expected_unit_price = intent
                .expected_unit_price
                .checked_add(1)
                .ok_or("price overflow")?;
            let reply = trade(
                players[index].client()?,
                &intent,
                wire::NpcIntentDisposition::Stale,
            )
            .await?;
            let after = snapshot(&mut players[index]).await?;
            if before != after {
                return Err(format!("{reply} before={before:?} after={after:?}").into());
            }
            Ok(())
        }
        .await;
        check(&format!("stale-price-{index}"), stale, failures);
        let result = round_trip(players, index, window).await;
        check(&format!("buy-sell-{index}"), result, failures);
    }
    let result = full_backpack(&mut players[0], &first).await;
    check("full-backpack", result, failures);
    let result = async {
        for (index, player) in players.iter_mut().enumerate() {
            talk(player.client()?, actors[index], "bye").await?;
            let (request, _) = knife_quote(&first, wire::NpcTradeSide::Buy)?;
            let before = snapshot(player).await?;
            let reply = trade(
                player.client()?,
                &request,
                wire::NpcIntentDisposition::NoConversation,
            )
            .await?;
            unchanged(player, before, &reply).await?;
        }
        Ok(())
    }
    .await;
    check("bye", result, failures);
    Ok(())
}

async fn round_trip(
    players: &mut [Bot<'_, '_>],
    index: usize,
    window: &wire::NpcTradeWindow,
) -> TestResult {
    let (buy, offer) = knife_quote(window, wire::NpcTradeSide::Buy)?;
    let (sell, _) = knife_quote(window, wire::NpcTradeSide::Sell)?;
    for (intent, increase) in [(buy, true), (sell, false)] {
        let before = snapshot(&mut players[index]).await?;
        let witness = snapshot(&mut players[1 - index]).await?;
        // Even insufficient funds must reach the server and retain its concrete reply.
        let reply = trade(
            players[index].client()?,
            &intent,
            wire::NpcIntentDisposition::Ok,
        )
        .await?;
        let count = |state: &Snapshot| {
            state
                .inventory
                .get(&offer.item_definition_ref)
                .copied()
                .unwrap_or(0)
        };
        let expected_count = if increase {
            count(&before).checked_add(u64::from(offer.count) * u64::from(intent.quantity))
        } else {
            count(&before).checked_sub(u64::from(offer.count) * u64::from(intent.quantity))
        }
        .ok_or("item count overflow")?;
        let expected_gold = if increase {
            before
                .gold
                .checked_sub(u64::from(intent.expected_unit_price) * u64::from(intent.quantity))
        } else {
            before
                .gold
                .checked_add(u64::from(intent.expected_unit_price) * u64::from(intent.quantity))
        }
        .ok_or_else(|| format!("{reply} impossible gold delta before={before:?}"))?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        bots::wait_observation(players, deadline, "MP05 trade inventory", |players, _| {
            let inventory = players[index]
                .session
                .as_ref()
                .and_then(DevClientSession::inventory)
                .ok_or("inventory absent after trade")?;
            let observed: u64 = inventory
                .entries
                .iter()
                .filter(|entry| entry.item_definition_ref.get() == offer.item_definition_ref)
                .map(|entry| u64::from(entry.count.get()))
                .sum();
            Ok(observed == expected_count)
        })
        .await
        .map_err(|error| format!("{reply}; {error}"))?;
        let after = snapshot(&mut players[index]).await?;
        let other = snapshot(&mut players[1 - index]).await?;
        if count(&after) != expected_count || after.gold != expected_gold || witness != other {
            return Err(format!("{reply} before={before:?} after={after:?} expected_count={expected_count} expected_gold={expected_gold} witness_before={witness:?} witness_after={other:?}").into());
        }
    }
    Ok(())
}

async fn relog(players: &mut [Bot<'_, '_>]) -> TestResult {
    let mut before = Vec::new();
    let mut sessions = Vec::new();
    for player in players.iter_mut() {
        before.push(snapshot(player).await?);
        sessions.push(player.client()?.join_snapshot().game_session_id);
    }
    bots::relog_all(players).await?;
    for (index, player) in players.iter_mut().enumerate() {
        let after = snapshot(player).await?;
        if before[index] != after
            || player.client()?.join_snapshot().game_session_id == sessions[index]
        {
            return Err(format!(
                "bot={index} relog before={:?} after={after:?} session={:?}",
                before[index],
                player.client()?.join_snapshot().game_session_id
            )
            .into());
        }
    }
    Ok(())
}

// Crystal sam.lua:238. Two throwing knives are a real stack, not two
// non-stackable club instances. Reject a converted catalogue with different prices.
fn knife_quote(
    window: &wire::NpcTradeWindow,
    side: wire::NpcTradeSide,
) -> TestResult<(wire::NpcTradeIntent, wire::NpcOffer)> {
    let reference = provisioning::item_reference(&provisioning::item_index()?, 3298)?;
    let (mut intent, offer) = quote(window, side, Some(reference.get()))?;
    let expected = if side == wire::NpcTradeSide::Buy {
        25
    } else {
        2
    };
    if offer.unit_price != expected || offer.count != 1 {
        return Err(format!(
            "Crystal sam.lua:238 expected knife price={expected} count=1; live={offer:?}"
        )
        .into());
    }
    intent.quantity = 2;
    Ok((intent, offer))
}

async fn unchanged(player: &mut Bot<'_, '_>, before: Snapshot, reply: &str) -> TestResult {
    let after = snapshot(player)
        .await
        .map_err(|error| format!("{reply}; {error}"))?;
    if before != after {
        return Err(
            format!("{reply}; refusal moved state before={before:?} after={after:?}").into(),
        );
    }
    Ok(())
}

async fn insufficient_gold(player: &mut Bot<'_, '_>, window: &wire::NpcTradeWindow) -> TestResult {
    let before = snapshot(player).await?;
    let (index, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, offer)| {
            offer.side == wire::NpcTradeSide::Buy && u64::from(offer.unit_price) > before.gold
        })
        .ok_or_else(|| {
            format!(
                "no unaffordable live quote; gold={} window={window:?}",
                before.gold
            )
        })?;
    let (npc_actor, catalogue_revision) = window.open.ok_or("window closed")?;
    let intent = wire::NpcTradeIntent {
        npc_actor,
        catalogue_revision,
        offer_index: u32::try_from(index)?,
        side: offer.side,
        quantity: 1,
        expected_unit_price: offer.unit_price,
    };
    let reply = trade(
        player.client()?,
        &intent,
        wire::NpcIntentDisposition::Rejected,
    )
    .await?;
    evidence(&format!(
        "MP05_REFUSAL insufficient-gold price={} gold={} {reply}; source:crates/gameplay/durability/src/durability/npc_trade.rs:962",
        offer.unit_price, before.gold
    ));
    unchanged(player, before, &reply).await
}

async fn unsupported_sale(player: &mut Bot<'_, '_>, window: &wire::NpcTradeWindow) -> TestResult {
    let before = snapshot(player).await?;
    let inventory = player.client()?.inventory().ok_or("inventory absent")?;
    let item = inventory
        .entries
        .iter()
        .chain(inventory.main_backpack.iter())
        .find(|entry| {
            !window.offers.iter().any(|offer| {
                offer.side == wire::NpcTradeSide::Sell
                    && offer.item_definition_ref == entry.item_definition_ref.get()
            })
        })
        .ok_or("no carried item outside Sam's buy list")?;
    let definition = item.item_definition_ref.get();
    // The ordinary wire only names projected offer indexes. If a buy-only offer
    // exists for this carried item, submit that actual quote with the Sell side.
    let selected = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, offer)| offer.item_definition_ref == definition);
    let (npc_actor, catalogue_revision) = window.open.ok_or("window closed")?;
    let (index, price) = selected.map_or((window.offers.len(), 0), |(index, offer)| {
        (index, offer.unit_price)
    });
    let intent = wire::NpcTradeIntent {
        npc_actor,
        catalogue_revision,
        offer_index: u32::try_from(index)?,
        side: wire::NpcTradeSide::Sell,
        quantity: 1,
        expected_unit_price: price,
    };
    let reply = trade(player.client()?, &intent, wire::NpcIntentDisposition::Stale).await?;
    unchanged(player, before, &reply).await?;
    if selected.is_none() {
        return Err(format!("unsupported carried definition={definition} cannot be named by the offer-index-only trade command; invalid offer refused: {reply}; source:crates/protocol-oteryn/src/npc_service.rs:146; source:crates/gameplay/dispatch/src/gameplay_transport/npc_command.rs:453").into());
    }
    Ok(())
}

async fn concurrent_buy(
    players: &mut [Bot<'_, '_>],
    windows: [&wire::NpcTradeWindow; 2],
) -> TestResult {
    let (a, offer_a) = knife_quote(windows[0], wire::NpcTradeSide::Buy)?;
    let (b, offer_b) = knife_quote(windows[1], wire::NpcTradeSide::Buy)?;
    let before = [
        snapshot(&mut players[0]).await?,
        snapshot(&mut players[1]).await?,
    ];
    let (left, right) = players.split_at_mut(1);
    let (reply_a, reply_b) = tokio::join!(
        trade(left[0].client()?, &a, wire::NpcIntentDisposition::Ok),
        trade(right[0].client()?, &b, wire::NpcIntentDisposition::Ok)
    );
    // Both exchanges are completed even when one player is refused.
    let replies = [reply_a, reply_b];
    let mut failures = Vec::new();
    for (index, (reply, offer)) in replies.into_iter().zip([offer_a, offer_b]).enumerate() {
        let result = async {
            let reply = reply?;
            let expected = before[index].inventory.get(&offer.item_definition_ref).copied().unwrap_or(0) + 2;
            let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
            bots::wait_observation(players, deadline, "MP05 concurrent inventory", |players, _| {
                let count: u64 = players[index].session.as_ref().and_then(DevClientSession::inventory).ok_or("inventory absent")?.entries.iter().filter(|entry| entry.item_definition_ref.get() == offer.item_definition_ref).map(|entry| u64::from(entry.count.get())).sum();
                Ok(count == expected)
            }).await.map_err(|error| format!("{reply}; {error}"))?;
            let after = snapshot(&mut players[index]).await?;
            if before[index].gold.checked_sub(50) != Some(after.gold) {
                return Err(format!("{reply}; concurrent bot={index} before={:?} after={after:?}; Crystal sam.lua:238 cost=2*25", before[index]).into());
            }
            TestResult::Ok(())
        }.await;
        if let Err(error) = result {
            failures.push(format!("bot={index} {error}"));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; ").into())
    }
}

// Read the authored capacity of this player's real backpack. This is an
// observation, never a fixture edit or a substitute for the wire purchase.
fn backpack_capacity(client: &DevClientSession) -> TestResult<usize> {
    let reference = client
        .inventory()
        .and_then(|inventory| inventory.main_backpack.as_ref())
        .ok_or("no main backpack")?
        .item_definition_ref;
    let index = provisioning::item_index()?;
    let (key, _) = index
        .definition_at(reference)
        .ok_or("backpack definition absent")?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/items/definitions");
    for entry in std::fs::read_dir(root)? {
        let path = entry?.path();
        if path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }
        let document: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
        if let Some(records) = document["records"].as_array() {
            for record in records {
                let definition = &record["definition"];
                if definition["identity"]["key"] == key {
                    let capacity =
                        definition["semantics"]["container"]["value"]["capacity"]["value"]
                            .as_u64()
                            .ok_or("authored backpack capacity absent")?;
                    return Ok(usize::try_from(capacity)?);
                }
            }
        }
    }
    Err(format!("backpack definition {key} not found").into())
}

async fn full_backpack(player: &mut Bot<'_, '_>, window: &wire::NpcTradeWindow) -> TestResult {
    // Crystal sam.lua:176: non-stackable club buy=5, sell=1. Fill real
    // backpack slots by ordinary purchases, then test one further club.
    let reference = provisioning::item_reference(&provisioning::item_index()?, 3270)?;
    let (intent, offer) = quote(window, wire::NpcTradeSide::Buy, Some(reference.get()))?;
    if offer.unit_price != 5 || offer.count != 1 {
        return Err(format!("Crystal club 5 gold; live={offer:?}").into());
    }
    let capacity = backpack_capacity(player.client()?)?;
    let mut last_reply = String::new();
    for _ in 0..=capacity + 3 {
        let before = snapshot(player).await?;
        let occupied = player
            .client()?
            .inventory()
            .ok_or("inventory absent")?
            .entries
            .len();
        if occupied > capacity {
            return Err(format!(
                "occupancy={occupied} exceeds authored capacity={capacity}; last_reply={last_reply}"
            )
            .into());
        }
        if occupied == capacity {
            let reply = trade(
                player.client()?,
                &intent,
                wire::NpcIntentDisposition::Rejected,
            )
            .await?;
            if before.gold < 5 {
                return Err(format!(
                    "full backpack but insufficient gold={} for isolated capacity refusal; {reply}",
                    before.gold
                )
                .into());
            }
            evidence(&format!(
                "MP05_REFUSAL full-backpack occupied={occupied} capacity={capacity} gold={} {reply}; source:crates/gameplay/durability/src/durability/npc_trade.rs:937",
                before.gold
            ));
            return unchanged(player, before, &reply).await;
        }
        // A refusal while filling is FAIL, including a weight or funds refusal;
        // neither establishes the requested full-backpack precondition.
        last_reply = trade(player.client()?, &intent, wire::NpcIntentDisposition::Ok).await?;
        let expected = before.inventory.get(&reference.get()).copied().unwrap_or(0) + 1;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        let client = player.client()?;
        tokio::time::timeout_at(deadline, async {
            while client.inventory().is_none_or(|inventory| {
                let count: u64 = inventory
                    .entries
                    .iter()
                    .filter(|entry| entry.item_definition_ref == reference)
                    .map(|entry| u64::from(entry.count.get()))
                    .sum();
                count != expected
            }) {
                client.service_liveness(Duration::from_millis(20)).await?;
            }
            TestResult::Ok(())
        })
        .await
        .map_err(|error| format!("{last_reply}; filling backpack: {error}"))??;
        let after = snapshot(player).await?;
        if before.gold.checked_sub(5) != Some(after.gold) {
            return Err(
                format!("{last_reply}; fill gold before={before:?} after={after:?}").into(),
            );
        }
    }
    Err(format!("backpack fill bound exhausted; last_reply={last_reply}").into())
}
