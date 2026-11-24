//! E2EF4: real-world economy/social probes, with fail-closed full-row evidence.
//! No fixture provisioning, SQL mutations, clock overrides or invented wire handles.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityRef};

use verdict::Row;

// Detail stays on the GAP lines, separate from the report's single-token evidence field.
// A failure is never turned into MISSING-SERVER merely because this journey is unfinished.
const EVIDENCE: &str = "source:apps/game-server/src/qualification/client_phases/full_e2ef4.rs:1";

const E01: &[&str] = &[
    "greet",
    "look",
    "buy",
    "sell",
    "sell-all",
    "deposit",
    "withdraw",
    "transfer",
    "relog-inventory-bank",
];
const E03: &[&str] = &[
    "gold-up",
    "platinum-down",
    "platinum-up",
    "crystal-down",
    "converter-charge",
    "converter-retirement",
    "magic-toggle",
    "magic-tick",
    "relog-coins-charges",
];
const E04: &[&str] = &[
    "request",
    "look",
    "accept",
    "cancel",
    "logout-race",
    "relog-custody",
];
const E05: &[&str] = &[
    "browse",
    "create",
    "cancel",
    "accept",
    "history",
    "relog-settlement",
];
const E07: &[&str] = &["depot", "inbox", "stow", "chosen-withdraw", "relog-custody"];
const S01: &[&str] = &[
    "invite",
    "join",
    "leadership",
    "shared-xp-toggle",
    "shared-xp-split",
    "leave",
];
const S03: &[&str] = &[
    "guild",
    "ranks",
    "motd",
    "chat",
    "bank",
    "wars",
    "relog-guild-bank",
];
const S09: &[&str] = &[
    "pz-logout",
    "fight-refusal",
    "nologout",
    "idle-warning",
    "idle-removal",
    "gameplay-resets-idle",
    "relog-position",
];
const S18: &[&str] = &[
    "say",
    "whisper",
    "yell",
    "range",
    "yell-level-premium",
    "mute",
    "fresh-login",
];

pub(super) async fn run(
    client: &mut Option<DevClientSession>,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    spawns: &oteryn_world_bundle::spawn::Table,
    rat_xp: i64,
) {
    // Each row reports even when another row has missing content or a refused command.
    // Socket waits are bounded; a timed-out session is not evidence of server absence.
    macro_rules! row {
        // The walk to the row's start (`locker`: the depot locker, `town`: the entry tile) is
        // travel outside the journey budget, as E01's walk to Sam is; each walked tile is
        // progress, so a stalled walk ends on the no-progress watchdog.
        (travel $to:ident, $id:literal, $actions:ident, $journey:ident $(, $arg:expr)*) => {{
            let mut row = Row::new("E2EF4", $id, $actions).evidence(EVIDENCE, EVIDENCE).gap_lines();
            let watch = row.watch();
            let result = if let Some(active) = client.as_mut() {
                let mut arrived = false;
                let walked = watch
                    .travel(
                        concat!($id, " walk to ", stringify!($to), " stalled;remaining-assertions-unproven"),
                        async {
                            let walked = $to(active, world).await;
                            arrived = true;
                            walked
                        },
                    )
                    .await;
                if !arrived {
                    // An interrupted command or frame cannot be resumed by another row.
                    *client = None;
                    walked
                } else if walked.is_err() {
                    walked
                } else {
                    row!(@journey client, row, watch, active, $journey $(, $arg)*)
                }
            } else {
                Err("recovery admission unavailable: prior session closed; Platform-issued grant input is absent; source:apps/game-server/src/qualification.rs:2720".into())
            };
            finish_row(client, row, result).await;
        }};
        (@journey $client:ident, $row:ident, $watch:ident, $active:ident, $journey:ident $(, $arg:expr)*) => {{
            let mut completed = false;
            let result = $watch
                .bound(
                    Duration::from_secs(240),
                    "native-journey-deadline;remaining-assertions-unproven",
                    async {
                        let result = $journey(&mut $row, $active $(, $arg)*).await;
                        completed = true;
                        result
                    },
                )
                .await;
            if !completed {
                // An interrupted command or frame cannot be resumed by another row.
                *$client = None;
            }
            result
        }};
        ($id:literal, $actions:ident, $journey:ident $(, $arg:expr)*) => {{
            let mut row = Row::new("E2EF4", $id, $actions).evidence(EVIDENCE, EVIDENCE).gap_lines();
            let watch = row.watch();
            let result = if let Some(active) = client.as_mut() {
                let mut completed = false;
                let result = watch.bound(
                    Duration::from_secs(240),
                    "native-journey-deadline;remaining-assertions-unproven",
                    async {
                        let result = $journey(&mut row, active $(, $arg)*).await;
                        completed = true;
                        result
                    },
                ).await;
                if !completed {
                    // An interrupted command or frame cannot be resumed by another row.
                    *client = None;
                }
                result
            } else {
                Err("recovery admission unavailable: prior session closed; Platform-issued grant input is absent; source:apps/game-server/src/qualification.rs:2720".into())
            };
            finish_row(client, row, result).await;
        }};
    }
    {
        let mut row = Row::new("E2EF4", "E01", E01)
            .evidence(EVIDENCE, EVIDENCE)
            .gap_lines();
        let result = if let Some(active) = client.as_mut() {
            let (result, completed) = shop_row(&mut row, active, world, npcs).await;
            if !completed {
                // An interrupted command or frame cannot be resumed by another row.
                *client = None;
            }
            result
        } else {
            Err("recovery admission unavailable: prior session closed; Platform-issued grant input is absent; source:apps/game-server/src/qualification.rs:2720".into())
        };
        finish_row(client, row, result).await;
    }
    row!("E03", E03, coins, world, spawns);
    row!("E04", E04, trade, clients, world, spawns);
    row!(travel locker, "E05", E05, market, clients, world, npcs, spawns);
    row!(travel locker, "E07", E07, storage, clients);
    row!(travel town, "S01", S01, party, clients, world, spawns, rat_xp);
    row!("S03", S03, guild);
    row!("S09", S09, logout, clients, world, spawns);
    row!("S18", S18, speech, clients, world);
    let Some(client) = client.as_mut() else {
        return;
    };
    // Restore the entry position expected by the pre-existing world phases.
    let restore: TestResult = async {
        let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
            at == TOWN
        })
        .ok_or("E2EF4 return route unavailable")?;
        walk(client, world, &path).await
    }
    .await;
    if let Err(error) = restore {
        evidence(&format!("E2EF4_GAP return {error}"));
    }
}

fn own(client: &DevClientSession) -> TestResult<EntityRef> {
    let entities = client.world_entities().ok_or("native entities absent")?;
    entities
        .iter()
        .find(|entity| &entity.entity.identity == entities.own_identity())
        .map(|entity| entity.entity)
        .ok_or_else(|| "native own actor absent".into())
}

async fn approach_position(
    client: &mut DevClientSession,
    other: TilePos,
    world: &BundleWorld,
) -> TestResult {
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at != other
            && at.floor == other.floor
            && at.x.abs_diff(other.x).max(at.y.abs_diff(other.y)) <= 1
    })
    .ok_or("real-peer approach route unavailable")?;
    walk(client, world, &path).await
}

fn inventory_units(client: &DevClientSession) -> TestResult<BTreeMap<u32, u64>> {
    let inventory = client.inventory().ok_or("native inventory absent")?;
    let mut units = BTreeMap::new();
    for item in inventory
        .entries
        .iter()
        .chain(inventory.main_backpack.iter())
        .chain(inventory.equipment.iter().map(|e| &e.item))
    {
        *units.entry(item.item_definition_ref.get()).or_default() += u64::from(item.count.get());
    }
    Ok(units)
}

async fn wait_inventory(
    client: &mut DevClientSession,
    expected: &BTreeMap<u32, u64>,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if &inventory_units(client)? == expected {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("native-inventory-quantity-deadline".into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
        client.take_events();
    }
}

async fn relog(client: &mut DevClientSession, clients: &SeamClients<'_>) -> TestResult {
    let old = client.join_snapshot().game_session_id;
    let (result, logout, attempts, busy_timed_out) = super::logout_retrying_busy(client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("durability-check logout refused: {logout}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
    }
    *client = connect_character(clients, 0x85, 1).await?;
    if client.join_snapshot().game_session_id == old {
        return Err("fresh-login reused old session".into());
    }
    Ok(())
}

async fn locker(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        near_locker(world, at)
    })
    .ok_or("no reachable production depot locker")?;
    walk(client, world, &path).await
}

/// Walks back to the entry tile before S01 approaches its peer from there.
async fn town(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at == TOWN
    })
    .ok_or("E2EF4 town route unavailable")?;
    walk(client, world, &path).await
}

/// Sam's authored wander radius: `oteryn:behavior.npc.sam` movement.wander.radius_tiles in
/// content/behaviors/behaviors-02500-02999.json. He is seldom on his placement tile.
const SAM_WANDER_RADIUS: u32 = 2;

/// E01 walks from the entry tile to Sam and back as travel, outside the 90 s journey budget:
/// at level 1 the walk to Sam alone took 87.6 s of it on main (node boot run 44884398508).
/// Each walked tile is progress, so a stalled walk still ends on the no-progress watchdog.
/// Returns the row result and whether the connection is still usable.
async fn shop_row(
    row: &mut Row,
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> (TestResult, bool) {
    let watch = row.watch();
    let entry = match tile(client.world_spatial().actor_position) {
        Ok(entry) => entry,
        Err(error) => return (Err(error), false),
    };
    let mut arrived = false;
    let actor = watch
        .travel(
            "E01 walk to Sam stalled;remaining-assertions-unproven",
            async {
                let actor = approach_sam(client, world, npcs).await;
                arrived = true;
                actor
            },
        )
        .await;
    if !arrived {
        return (actor.map(|_| ()), false);
    }
    let mut completed = false;
    let result = match actor {
        Ok(actor) => {
            watch
                .bound(
                    Duration::from_secs(90),
                    "native-journey-deadline;remaining-assertions-unproven",
                    async {
                        let result = shop(row, client, actor).await;
                        completed = true;
                        result
                    },
                )
                .await
        }
        Err(error) => {
            completed = true;
            Err(error)
        }
    };
    if !completed {
        return (result, false);
    }
    // The rows after E01 start on the entry tile, as they did before Sam was reachable.
    let back = watch
        .travel("E01 return walk stalled", async {
            let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
                at == entry
            })
            .ok_or("E01 return route unavailable")?;
            walk(client, world, &path).await?;
            if tile(client.world_spatial().actor_position)? != entry {
                return Err("E01 return did not reach the entry tile".into());
            }
            Ok(())
        })
        .await;
    if let Err(error) = &back {
        evidence(&format!("E2EF4_GAP E01 return {error}"));
    }
    let reusable = back.is_ok();
    (result.and(back), reusable)
}

async fn approach_sam(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult<EntityRef> {
    let basis = spatial_basis::current(client)?;
    let npc = npcs
        .npcs
        .iter()
        .find(|npc| npc.key == "oteryn:npc.sam")
        .ok_or("content:oteryn:npc.sam absent")?;
    // Next to the placement, Sam stays within TALK_RANGE_TILES (4) while he wanders.
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        npc.placements
            .iter()
            .any(|p| p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 1)
    })
    .ok_or("content:oteryn:npc.sam unreachable")?;
    walk(client, world, &path).await?;
    let entities = client.world_entities().ok_or("native entities absent")?;
    let near: Vec<_> = entities
        .iter()
        .filter(|entity| {
            entity.kind == oteryn_dev_client::EntityKind::Npc
                && npc.placements.iter().any(|p| {
                    entity.position.floor == basis.floor(p.floor)
                        && entity.position.x.abs_diff(i32::from(p.x)) <= SAM_WANDER_RADIUS
                        && entity.position.y.abs_diff(i32::from(p.y)) <= SAM_WANDER_RADIUS
                })
        })
        .collect();
    let named: Vec<_> = near
        .iter()
        .copied()
        .filter(|entity| {
            entity
                .display_name
                .as_ref()
                .is_some_and(|name| name.as_str().is_ok_and(|name| name == "Sam"))
        })
        .collect();
    match (near.as_slice(), named.as_slice()) {
        ([sam], _) | (_, [sam]) => Ok(sam.entity),
        ([], _) => Err("Sam actor not visible within his authored wander radius".into()),
        _ => Err(format!(
            "{} NPC actors within Sam's wander radius and none uniquely named Sam",
            near.len()
        )
        .into()),
    }
}

async fn shop(row: &mut Row, client: &mut DevClientSession, actor: EntityRef) -> TestResult {
    use oteryn_dev_client::client_runtime::npc_shop as runtime;
    use oteryn_protocol_oteryn::{npc_service as wire, npc_shop as shop};
    let greeting = super::client_row_e01::talk(client, actor, "hi", None).await?;
    if greeting.open.map(|(npc, _)| npc) != Some(actor) {
        return Err("greet: Sam did not open the native conversation".into());
    }
    row.pass("greet", "native-conversation-open-for-Sam");
    // Trade can open a window without emitting a second conversation delta.
    tokio::time::sleep(Duration::from_millis(260)).await;
    let trade = runtime::talk(
        client.session_mut(),
        &wire::NpcTalkIntent {
            npc_actor: actor,
            text: "trade".into(),
        },
    )
    .await?;
    if trade != wire::NpcIntentDisposition::Ok {
        return Err(format!("Sam refused trade: {trade:?}").into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let details = client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::trade_details::CAPABILITY_TRADE_DETAILS_V1);
    let window = wire::decode_npc_trade_window_for(&delta.payload, details)
        .map_err(|e| format!("shop window: {e:?}"))?;
    if window.open.map(|(npc, _)| npc) != Some(actor) {
        return Err("trade window did not open for Sam".into());
    }
    let (index, offer) = window
        .offers
        .iter()
        .enumerate()
        .filter(|(_, o)| o.side == wire::NpcTradeSide::Buy)
        .min_by_key(|(_, offer)| {
            let reversible = window.offers.iter().any(|candidate| {
                candidate.side == wire::NpcTradeSide::Sell
                    && candidate.item_definition_ref == offer.item_definition_ref
            });
            (!reversible, offer.unit_price)
        })
        .ok_or("Sam has no buy offer")?;
    let result = runtime::send(
        client.session_mut(),
        &runtime::NpcShopCommand(shop::NpcShopIntent {
            npc_actor: actor,
            catalogue_revision: window.open.ok_or("trade window closed")?.1,
            offer_index: u32::try_from(index)?,
            side: offer.side,
            action: shop::NpcShopAction::Look,
            expected_unit_price: offer.unit_price,
        }),
    )
    .await?;
    if !result
        .result
        .is_some_and(|r| r.disposition == wire::NpcIntentDisposition::Ok && !r.look_text.is_empty())
    {
        return Err("shop look did not return a description".into());
    }
    row.pass("look", "native-offer-description-nonempty");
    let sell = window.offers.iter().enumerate().find(|(_, candidate)| {
        candidate.side == wire::NpcTradeSide::Sell
            && candidate.item_definition_ref == offer.item_definition_ref
    });
    let revision = window.open.ok_or("trade window closed")?.1;
    let definition = offer.item_definition_ref;
    let before = inventory_units(client)?;
    let quantity = if sell.is_some() { 2 } else { 1 };
    let result = runtime::buy(
        client.session_mut(),
        &wire::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision: revision,
            offer_index: u32::try_from(index)?,
            side: offer.side,
            quantity,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if result != wire::NpcIntentDisposition::Ok {
        return Err(format!("buy: Sam refused {result:?} for {quantity} units at {}; need sufficient legitimately earned spendable gold; source:crates/gameplay/dispatch/src/gameplay_transport/npc_trade_command.rs:185", offer.unit_price).into());
    }
    let after_buy = wait_shop_delta(
        client,
        &before,
        definition,
        i64::from(quantity),
        -i64::from(offer.unit_price) * i64::from(quantity),
    )
    .await?;
    row.pass("buy", "exact-item-increase-and-gold-debit");
    let Some((sell_index, sell)) = sell else {
        return Err("sell: Sam has no purchase quote for the bought item; need a reachable buyer for this definition; source:crates/gameplay/dispatch/src/gameplay_transport/npc_trade_command.rs:185".into());
    };
    let result = runtime::buy(
        client.session_mut(),
        &wire::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision: revision,
            offer_index: u32::try_from(sell_index)?,
            side: sell.side,
            quantity: 1,
            expected_unit_price: sell.unit_price,
        },
    )
    .await?;
    if result != wire::NpcIntentDisposition::Ok {
        return Err(format!("sell: Sam refused {result:?}").into());
    }
    let after_sell = wait_shop_delta(
        client,
        &after_buy,
        definition,
        -1,
        i64::from(sell.unit_price),
    )
    .await?;
    row.pass("sell", "exact-item-decrease-and-gold-credit");
    let remaining = after_sell.get(&definition).copied().unwrap_or_default();
    let result = runtime::send(
        client.session_mut(),
        &runtime::NpcShopCommand(shop::NpcShopIntent {
            npc_actor: actor,
            catalogue_revision: revision,
            offer_index: u32::try_from(sell_index)?,
            side: sell.side,
            action: shop::NpcShopAction::SellAll,
            expected_unit_price: sell.unit_price,
        }),
    )
    .await?;
    if !result
        .result
        .is_some_and(|r| r.disposition == wire::NpcIntentDisposition::Ok)
    {
        return Err("sell-all: Sam refused the remaining carried quantity".into());
    }
    let remaining = i64::try_from(remaining)?;
    wait_shop_delta(
        client,
        &after_sell,
        definition,
        -remaining,
        remaining
            .checked_mul(i64::from(sell.unit_price))
            .ok_or("sell-all gold overflow")?,
    )
    .await?;
    row.pass("sell-all", "all-carried-units-sold-and-exact-gold-credit");
    Err("deposit/withdraw/transfer/relog-inventory-bank: need a reachable Bank service; authored banker Paulie (32115,32188,8) is outside the reference starter regions; source:content/world/npc-placements/placements-00000-00055.json:1".into())
}

fn quantity_delta(before: u64, after: u64, expected: i64) -> bool {
    i128::from(after) - i128::from(before) == i128::from(expected)
}

async fn wait_shop_delta(
    client: &mut DevClientSession,
    before: &BTreeMap<u32, u64>,
    definition: u32,
    items: i64,
    gold: i64,
) -> TestResult<BTreeMap<u32, u64>> {
    let definitions = provisioning::item_index()?;
    let coin_refs = [3031, 3035, 3043].map(|id| provisioning::item_reference(&definitions, id));
    let [gold_ref, platinum_ref, crystal_ref] = coin_refs;
    let refs = [gold_ref?.get(), platinum_ref?.get(), crystal_ref?.get()];
    let value = |inventory: &BTreeMap<u32, u64>| -> u64 {
        refs.into_iter()
            .zip([1, 100, 10_000])
            .map(|(reference, rate)| inventory.get(&reference).copied().unwrap_or_default() * rate)
            .sum()
    };
    let count =
        |inventory: &BTreeMap<u32, u64>| inventory.get(&definition).copied().unwrap_or_default();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let after = inventory_units(client)?;
        if quantity_delta(count(before), count(&after), items)
            && quantity_delta(value(before), value(&after), gold)
        {
            return Ok(after);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("trade inventory delta expected-items={items} expected-gold={gold}; items={}->{} gold={}->{}", count(before), count(&after), value(before), value(&after)).into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
        client.take_events();
    }
}

type CoinCounts = [u64; 3];

const COIN_STEPS: [([u64; 3], [u64; 3]); 4] = [
    ([100, 0, 0], [0, 1, 0]),
    ([0, 1, 0], [100, 0, 0]),
    ([0, 100, 0], [0, 0, 1]),
    ([0, 0, 1], [0, 100, 0]),
];

fn exchange_counts(
    before: CoinCounts,
    (input, output): (CoinCounts, CoinCounts),
) -> Option<CoinCounts> {
    let mut after = before;
    for (count, (removed, added)) in after.iter_mut().zip(input.into_iter().zip(output)) {
        *count = count.checked_sub(removed)?.checked_add(added)?;
    }
    Some(after)
}

async fn coins(
    row: &mut Row,
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult {
    use super::client_row_e02::{backpack_coins, coin_refs, wait_coins};
    let refs = coin_refs()?;
    if backpack_coins(client, refs)?[0] < 100 {
        let hunt: TestResult = async {
            travel_to_spawn(client, world, spawns, "oteryn:creature.rat").await?;
            client_hunt::hunt(client).await
        }
        .await;
        if let Err(error) = hunt {
            evidence(&format!("E2EF4_GAP E03 gold-acquisition: {error}"));
        }
        town(client, world).await?;
    }
    let mut before = backpack_coins(client, refs)?;
    let sources = [refs[0], refs[1], refs[1], refs[2]];
    for (((step, plan), source), quantity) in E03
        .iter()
        .zip(COIN_STEPS)
        .zip(sources)
        .zip([100, 1, 100, 1])
    {
        let item = client.inventory().and_then(|inventory| {
            inventory.entries.iter().find(|item| {
                item.item_definition_ref == source
                    && match *step {
                        "platinum-down" => item.count.get() < 100,
                        "crystal-down" => item.count.get() >= quantity,
                        _ => item.count.get() == quantity,
                    }
            })
        }).copied().ok_or_else(|| format!(
            "{step}: need an eligible earned backpack stack definition={source}; observed={before:?}; source:crates/gameplay/economy/src/coin_exchange.rs:132"
        ))?;
        let expected = exchange_counts(before, plan)
            .ok_or("coin use denomination delta exceeds observed backpack counts")?;
        let result = client.use_item(item.handle).await?;
        if result.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
            return Err(format!("{step}: use refused {:?}", result.disposition).into());
        }
        wait_coins(client, refs, expected, step).await?;
        row.pass(step, "exact-backpack-coin-delta");
        before = expected;
    }
    Err("converter-charge/converter-retirement/magic-toggle/magic-tick/relog-coins-charges: need legitimately acquired gold and magic gold converters plus eligible coin stacks; source:crates/gameplay/dispatch/src/gameplay_transport/coin_use.rs:24".into())
}

async fn inventory_after_trade(
    client: &mut DevClientSession,
    expected: &BTreeMap<u32, u64>,
) -> TestResult {
    // Read through the race's pending pushes before accepting an unchanged
    // inventory, which otherwise could just be the pre-race projection.
    let reply = client.session_mut().attack_target(None).await?;
    if reply.status != CommandStatus::Accepted
        || reply.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
    {
        return Err(format!("logout-race: inventory readback refused {reply:?}").into());
    }
    wait_inventory(client, expected).await
}

async fn trade(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult {
    evidence(&format!("E2EF4_GAP E04 {}", platform_admission_gap()));
    use oteryn_dev_client::{
        client_runtime::player_trade as runtime, client_ui::player_trade as ui,
    };
    // The cancel is still exercised when the bootstrap has no transferable leaf.
    let mut panel = ui::PlayerTradePanel {
        selected: true,
        ..Default::default()
    };
    let command = ui::request(&mut panel, ui::Intent::Cancel).ok_or("trade cancel input")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    if reply.state.trade_id.is_some() {
        return Err("cancel retained open trade".into());
    }
    // Cancelling no active trade does not qualify the row's cancellation action.
    let mut peer = keep_alive(client, connect_character(clients, 0x81, 0)).await?;
    let mut peer_logged_out = false;
    let result: TestResult = async {
        let item = keep_alive(&mut peer, acquire_leaf(client, world, spawns, None)).await?;
        let counter = keep_alive(
            client,
            acquire_leaf(&mut peer, world, spawns, Some(item.item_definition_ref)),
        )
        .await?;
        // The looting trip can leave the peer in combat. Reach a logout-capable
        // PZ before arming the concurrent accept and logout probe.
        keep_alive(client, return_to_pz(&mut peer, world)).await?;
        let position = tile(peer.world_spatial().actor_position)?;
        keep_alive(&mut peer, approach_position(client, position, world)).await?;
        let before_main = inventory_units(client)?;
        let before_peer = inventory_units(&peer)?;
        let id = open_reciprocal_trade(client, &mut peer, item, counter).await?;
        row.pass("request", "real-peer-offer-and-counter-opened");
        let command = runtime::PlayerTradeCommand::new(runtime::player_trade::Intent::Look {
            trade_id: id,
            counter_offer: false,
        })
        .ok_or("trade look encoding")?;
        let reply = runtime::send(client.session_mut(), &command).await?;
        if reply.state.look.is_empty() {
            return Err("offered leaf look was empty".into());
        }
        row.pass("look", "offered-leaf-description");
        let command = runtime::PlayerTradeCommand::new(runtime::player_trade::Intent::Cancel)
            .ok_or("trade cancel encoding")?;
        let reply = runtime::send(client.session_mut(), &command).await?;
        if reply.result != runtime::player_trade::ResultCode::Cancelled || reply.state.phase != 4 {
            return Err("active trade cancellation did not close escrow".into());
        }
        peer.session_mut()
            .read_gated_delta(runtime::player_trade::STATE_DOMAIN_PLAYER_TRADE, 1)
            .await?;
        wait_inventory(client, &before_main).await?;
        wait_inventory(&mut peer, &before_peer).await?;
        row.pass(
            "cancel",
            "active-offer-closed-and-both-inventories-retained",
        );
        let expected_main = swapped_inventory(before_main, item, counter)?;
        let expected_peer = swapped_inventory(before_peer, counter, item)?;
        let id = open_reciprocal_trade(client, &mut peer, item, counter).await?;
        let accept = runtime::PlayerTradeCommand::new(runtime::player_trade::Intent::Accept {
            trade_id: id,
        })
        .ok_or("trade accept encoding")?;
        let first = runtime::send(client.session_mut(), &accept).await?;
        if first.result != runtime::player_trade::ResultCode::Accepted {
            return Err(format!("accept: first result={:?}", first.result).into());
        }
        let domain = runtime::player_trade::STATE_DOMAIN_PLAYER_TRADE;
        peer.session_mut().read_gated_delta(domain, 1).await?;
        let second = runtime::send(peer.session_mut(), &accept).await?;
        if second.result != runtime::player_trade::ResultCode::Completed || second.state.phase != 4
        {
            return Err(format!(
                "accept: second result={:?} phase={}",
                second.result, second.state.phase
            )
            .into());
        }
        client.session_mut().read_gated_delta(domain, 1).await?;
        wait_inventory(client, &expected_main).await?;
        wait_inventory(&mut peer, &expected_peer).await?;
        row.pass("accept", "both-accepts-and-exact-exchanged-custody");

        let item = trade_leaf(client, None).ok_or("logout-race: no carried leaf after exchange")?;
        let counter = trade_leaf(&peer, Some(item.item_definition_ref))
            .ok_or("logout-race: need distinguishable counter leaf after exchange")?;
        let raced_main = swapped_inventory(expected_main.clone(), item, counter)?;
        let raced_peer = swapped_inventory(expected_peer.clone(), counter, item)?;
        let id = open_reciprocal_trade(client, &mut peer, item, counter).await?;
        let accept = runtime::PlayerTradeCommand::new(runtime::player_trade::Intent::Accept {
            trade_id: id,
        })
        .ok_or("trade race encoding")?;
        if runtime::send(peer.session_mut(), &accept).await?.result
            != runtime::player_trade::ResultCode::Accepted
        {
            return Err("logout-race: first accept did not arm the race".into());
        }
        client.session_mut().read_gated_delta(domain, 1).await?;

        let (logout, final_accept) = tokio::join!(
            peer.session_mut().logout(),
            runtime::send(client.session_mut(), &accept),
        );
        let logout = logout?;
        peer_logged_out =
            logout.result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted);
        let accept = final_accept?;
        let (main, other) = match accept.result {
            runtime::player_trade::ResultCode::Completed => (raced_main, raced_peer),
            runtime::player_trade::ResultCode::Stale
            | runtime::player_trade::ResultCode::Cancelled => (expected_main, expected_peer),
            result => return Err(format!("logout-race: unexpected final accept {result:?}").into()),
        };
        if peer_logged_out {
            let old = peer.join_snapshot().game_session_id;
            let fresh = keep_alive(client, connect_character(clients, 0x81, 0)).await?;
            peer = fresh;
            peer_logged_out = false;
            let observed = inventory_units(&peer)?;
            if peer.join_snapshot().game_session_id == old || observed != other {
                return Err(format!("logout-race: fresh peer session={:?}, old={old:?}, inventory={observed:?}, expected={other:?}", peer.join_snapshot().game_session_id).into());
            }
        } else {
            if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Busy) {
                return Err(format!("logout-race: unexpected logout {logout:?}").into());
            }
            keep_alive(client, inventory_after_trade(&mut peer, &other)).await?;
        }
        keep_alive(&mut peer, inventory_after_trade(client, &main)).await?;
        row.pass(
            "logout-race",
            format!("accept={:?} logout={:?} both-inventories-exact", accept.result, logout.result),
        );
        Err(platform_admission_gap())
    }
    .await;
    if peer_logged_out {
        return result;
    }
    keep_alive(client, return_to_pz(&mut peer, world)).await?;
    let (cleanup_result, cleanup, attempts, busy_timed_out) =
        keep_alive(client, super::logout_retrying_busy(&mut peer)).await?;
    if cleanup_result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("trade peer cleanup logout refused: {cleanup}; attempts={attempts}; busy_timed_out={busy_timed_out}; journey={result:?}").into());
    }
    result
}

async fn acquire_leaf(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    exclude: Option<std::num::NonZeroU32>,
) -> TestResult<oteryn_protocol_oteryn::item_view::ItemEntry> {
    for creature in ["oteryn:creature.rat", "oteryn:creature.poison_spider"] {
        if let Some(item) = trade_leaf(client, exclude) {
            return Ok(item);
        }
        travel_to_spawn(client, world, spawns, creature).await?;
        client_hunt::hunt(client).await?;
    }
    trade_leaf(client, exclude).ok_or("acquisition: authored rat and spider hunts yielded no eligible carried leaf; source:apps/game-server/src/qualification/client_phases/hunt.rs:105".into())
}

fn trade_leaf(
    client: &DevClientSession,
    exclude: Option<std::num::NonZeroU32>,
) -> Option<oteryn_protocol_oteryn::item_view::ItemEntry> {
    client
        .inventory()?
        .entries
        .iter()
        .find(|item| Some(item.item_definition_ref) != exclude)
        .copied()
}

async fn open_reciprocal_trade(
    client: &mut DevClientSession,
    peer: &mut DevClientSession,
    item: oteryn_protocol_oteryn::item_view::ItemEntry,
    counter: oteryn_protocol_oteryn::item_view::ItemEntry,
) -> TestResult<[u8; 16]> {
    use oteryn_dev_client::client_runtime::player_trade as runtime;
    use runtime::player_trade::{Intent, ResultCode, STATE_DOMAIN_PLAYER_TRADE};
    let command = runtime::PlayerTradeCommand::new(Intent::Offer {
        partner: own(peer)?,
        item: item.handle.get(),
    })
    .ok_or("trade reopen encoding")?;
    let reopened = runtime::send(client.session_mut(), &command).await?;
    if reopened.result != ResultCode::Offered
        || reopened.state.phase != 1
        || reopened.state.trade_id.is_some()
    {
        return Err(format!(
            "offer: expected initial offer without pair ID, observed {:?}",
            reopened.state
        )
        .into());
    }
    peer.session_mut()
        .read_gated_delta(STATE_DOMAIN_PLAYER_TRADE, 1)
        .await?;
    let command = runtime::PlayerTradeCommand::new(Intent::Offer {
        partner: own(client)?,
        item: counter.handle.get(),
    })
    .ok_or("counter offer encoding")?;
    let ready = runtime::send(peer.session_mut(), &command).await?;
    if ready.result != ResultCode::Offered || ready.state.phase != 2 {
        return Err(format!(
            "accept: peer counter offer did not ready the trade: {:?}",
            ready.result
        )
        .into());
    }
    let id = ready
        .state
        .trade_id
        .ok_or("counter: READY did not supply a pair ID")?;
    let delta = client
        .session_mut()
        .read_gated_delta(STATE_DOMAIN_PLAYER_TRADE, 1)
        .await?;
    let own_state = runtime::player_trade::decode_state_for(
        &delta.payload,
        client.session_mut().is_selected(501),
    )
    .map_err(|error| format!("ready counterpart state: {error:?}"))?;
    if own_state.trade_id != Some(id) || own_state.phase != 2 {
        return Err("counter: participants received different READY trade IDs".into());
    }
    Ok(id)
}

fn swapped_inventory(
    mut counts: BTreeMap<u32, u64>,
    sent: oteryn_protocol_oteryn::item_view::ItemEntry,
    received: oteryn_protocol_oteryn::item_view::ItemEntry,
) -> TestResult<BTreeMap<u32, u64>> {
    let key = sent.item_definition_ref.get();
    let available = counts
        .get_mut(&key)
        .ok_or("trade source absent from inventory")?;
    *available = available
        .checked_sub(u64::from(sent.count.get()))
        .ok_or("trade source underflow")?;
    if *available == 0 {
        counts.remove(&key);
    }
    *counts
        .entry(received.item_definition_ref.get())
        .or_default() += u64::from(received.count.get());
    Ok(counts)
}

async fn fund_market(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    spawns: &oteryn_world_bundle::spawn::Table,
    required: u64,
) -> TestResult {
    use super::client_row_e02::{approach, backpack_coins, coin_refs};
    use super::full_row_e06::bank_balance;

    let before = bank_balance(clients, 1).await?;
    let needed = required.saturating_sub(before);
    if needed == 0 {
        return Ok(());
    }
    let gap = "market-funding: no reachable authored banker Paulie for earned gold deposit; source:content/world/npc-placements/placements-00000-00055.json:1";
    let banker = npcs
        .npcs
        .iter()
        .find(|npc| npc.key == "oteryn:npc.paulie")
        .ok_or(gap)?;
    if route(world, tile(client.world_spatial().actor_position)?, |at| {
        banker
            .placements
            .iter()
            .any(|p| p.floor == at.floor && p.x.abs_diff(at.x).max(p.y.abs_diff(at.y)) <= 3)
    })
    .is_none()
    {
        return Err(gap.into());
    }

    let refs = coin_refs()?;
    for _ in 0..8 {
        if backpack_coins(client, refs)?[0] >= needed {
            break;
        }
        travel_to_spawn(client, world, spawns, "oteryn:creature.rat").await?;
        client_hunt::hunt(client).await?;
    }
    if backpack_coins(client, refs)?[0] < needed {
        return Err(format!("market-funding: rat loot did not supply {needed} backpack gold for the Market fee and escrow").into());
    }
    let actor = approach(client, world, banker).await?;
    super::client_row_e01::talk(client, actor, "hi", None).await?;
    let text = format!("deposit {needed} gold");
    let quote = super::client_row_e01::talk(client, actor, &text, None).await?;
    if !quote
        .lines
        .iter()
        .any(|line| line == &format!("Do you want to deposit {needed} gold?"))
    {
        return Err(format!(
            "market-funding: banker did not quote {needed} gold: {:?}",
            quote.lines
        )
        .into());
    }
    super::client_row_e01::talk(client, actor, "yes", None).await?;
    if bank_balance(clients, 1).await? != required {
        return Err(
            "market-funding: accepted dialogue did not credit exact required bank gold".into(),
        );
    }
    locker(client, world).await
}

async fn market(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult {
    evidence(&format!("E2EF4_GAP E05 {}", platform_admission_gap()));
    use oteryn_dev_client::{client_runtime::market as runtime, client_ui::market as ui};
    // The row walked to the depot locker before this journey.
    let mut panel = ui::MarketPanel {
        selected: true,
        ..Default::default()
    };
    let command = ui::request(&mut panel, ui::Intent::Wares).ok_or("market wares request input")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    if !ui::apply_result(&mut panel, &reply) {
        return Err(format!("browse: market wares: {:?}", reply.reply).into());
    }
    let definition = panel
        .rows
        .first()
        .ok_or("browse: native wares catalogue is empty")?
        .definition;
    panel.definition = definition;
    let command = ui::request(&mut panel, ui::Intent::Browse).ok_or("market book request input")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    if !ui::apply_result(&mut panel, &reply) {
        return Err(format!(
            "browse: advertised marketable definition {definition} refused: {:?}",
            reply.reply
        )
        .into());
    }
    row.pass("browse", "native-wares-and-advertised-item-book-decoded");
    let fee = oteryn_gameplay_economy::market::placement_fee(1, 1)
        .map_err(|error| format!("market fee: {error:?}"))?;
    fund_market(client, clients, world, npcs, spawns, 2 * fee + 1).await?;
    panel.sell = false;
    let command = ui::request(&mut panel, ui::Intent::CreateOffer).ok_or("market create input")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    if !ui::apply_result(&mut panel, &reply) {
        return Err(format!(
            "market-create-refused:{:?};no-minted-funds-or-premium",
            reply.reply
        )
        .into());
    }
    let id = reply.reply.ok_or("market create receipt missing")?.offer;
    let command = ui::request(&mut panel, ui::Intent::OwnOffers).ok_or("market own input")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    if !ui::apply_result(&mut panel, &reply)
        || !panel.rows.iter().any(|r| {
            r.offer == id && r.amount == 1 && r.price == 1 && r.definition == panel.definition
        })
    {
        return Err("created market offer absent or quantity/price differ".into());
    }
    row.pass("create", "own-offers-exact-id-definition-amount-price");
    panel.selected_offer = Some(id);
    let command = ui::request(&mut panel, ui::Intent::CancelOffer).ok_or("market cancel input")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    if !ui::apply_result(&mut panel, &reply) {
        return Err("market cancellation refused".into());
    }
    let command = ui::request(&mut panel, ui::Intent::OwnOffers).ok_or("market verify cancel")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    if !ui::apply_result(&mut panel, &reply) || panel.rows.iter().any(|r| r.offer == id) {
        return Err("cancelled offer remains active".into());
    }
    row.pass("cancel", "cancelled-id-absent-from-own-offers");
    let command = ui::request(&mut panel, ui::Intent::History).ok_or("market history input")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    if !ui::apply_result(&mut panel, &reply) || !panel.rows.iter().any(|entry| entry.offer == id) {
        return Err("history: cancelled offer absent from personal history".into());
    }
    let mut seller = keep_alive(client, connect_character(clients, 0x87, 0)).await?;
    let journey = market_settle(row, client, &mut seller, clients, world, spawns).await;
    let (result, reply, attempts, timeout) = logout_retrying_busy(&mut seller).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("market seller cleanup: {reply}; attempts={attempts}; busy_timed_out={timeout}; journey={journey:?}").into());
    }
    journey
}

async fn market_settle(
    row: &mut Row,
    buyer: &mut DevClientSession,
    seller: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult {
    use super::full_row_e06::{bank_balance, command};
    use super::full_row_e08::depot_intent;
    use super::full_row_e09::{depot_search, inbox};
    use oteryn_protocol_oteryn::{depot, market::MarketRequest};

    let item = keep_alive(buyer, acquire_leaf(seller, world, spawns, None)).await?;
    keep_alive(buyer, locker(seller, world)).await?;
    let definitions = provisioning::item_index()?;
    let (key, _) = definitions
        .definition_at(item.item_definition_ref)
        .ok_or("accept: seller item definition is absent from served index")?;
    let client_id = key
        .strip_prefix("oteryn:item.tibia.i")
        .ok_or("accept: seller item has no market client id")?
        .parse()?;
    let definition = item.item_definition_ref.get();
    let market_request = |action, definition, offer, amount, price| MarketRequest {
        action,
        definition,
        offer,
        amount,
        price,
        anonymous: false,
        page: 0,
    };
    let book = command(seller, market_request(2, definition, [0; 16], 0, 0)).await?;
    if book.code != 1 {
        return Err(format!(
            "accept: seller leaf market book refused code={}; need marketable carried item",
            book.code
        )
        .into());
    }
    let opened = depot_intent(
        seller,
        depot::DepotIntent {
            town: None,
            box_number: 1,
            page: 0,
            action: depot::DepotAction::Browse,
        },
    )
    .await?;
    if opened.outcome != depot::DepotOutcome::Opened || opened.town.is_none() {
        return Err(format!("accept: seller depot browse refused {opened:?}").into());
    }
    let deposit = depot_intent(
        seller,
        depot::DepotIntent {
            town: opened.town,
            box_number: 1,
            page: 0,
            action: depot::DepotAction::Deposit(item.handle),
        },
    )
    .await?;
    if deposit.outcome != depot::DepotOutcome::Moved {
        return Err(format!("accept: seller depot deposit refused {deposit:?}").into());
    }
    let before = inbox(&depot_search(buyer, client_id).await?);
    let seller_gold = bank_balance(clients, 0).await?;
    let buyer_gold = bank_balance(clients, 1).await?;
    let quote = command(buyer, market_request(5, definition, [0; 16], 1, 1)).await?;
    if quote.code != 1 {
        return Err(format!("accept: funded buy offer refused code={}; need one gold escrow plus Market fee and current Premium", quote.code).into());
    }
    let accepted = command(seller, market_request(8, 0, quote.offer, 1, 0)).await?;
    if accepted.code != 1 {
        return Err(format!(
            "accept: cross-account seller refused code={}",
            accepted.code
        )
        .into());
    }
    let after = inbox(&depot_search(buyer, client_id).await?);
    let fee = oteryn_gameplay_economy::market::placement_fee(1, 1)
        .map_err(|error| format!("market fee: {error:?}"))?;
    let paid = bank_balance(clients, 0).await?;
    let charged = bank_balance(clients, 1).await?;
    if before.checked_add(1) != Some(after)
        || seller_gold.checked_add(1) != Some(paid)
        || buyer_gold.checked_sub(fee + 1) != Some(charged)
    {
        return Err(format!("accept: settlement custody mismatch inbox={before}->{after} seller={seller_gold}->{paid} buyer={buyer_gold}->{charged}").into());
    }
    row.pass(
        "accept",
        "cross-account-one-unit-inbox-and-exact-bank-deltas",
    );
    let history_request = market_request(4, 0, [0; 16], 0, 0);
    let buyer_history = command(buyer, history_request).await?;
    let seller_history = command(seller, history_request).await?;
    if !settled_offer(&buyer_history, quote.offer, definition)
        || !settled_offer(&seller_history, quote.offer, definition)
    {
        return Err("history: exact settled unit missing from a participant's history".into());
    }
    row.pass("history", "both-histories-contain-exact-settled-offer");
    Err(platform_admission_gap())
}

fn settled_offer(
    reply: &oteryn_protocol_oteryn::market::MarketReply,
    offer: [u8; 16],
    definition: u32,
) -> bool {
    reply.code == 1
        && reply.action == 4
        && reply.rows.iter().any(|entry| {
            entry.offer == offer
                && entry.definition == definition
                && entry.amount == 1
                && entry.price == 1
                && entry.state == 2
        })
}

async fn storage(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult {
    use oteryn_dev_client::{client_runtime::stash as runtime, client_ui::stash as ui};
    use oteryn_protocol_oteryn::depot;
    // The row walked to the depot locker before this journey.
    let browse = super::full_row_e08::depot_intent(
        client,
        depot::DepotIntent {
            town: None,
            box_number: 1,
            page: 0,
            action: depot::DepotAction::Browse,
        },
    )
    .await?;
    if browse.outcome != depot::DepotOutcome::Opened || browse.town.is_none() {
        return Err(format!("depot browse: {browse:?}").into());
    }
    let leaf = client
        .inventory()
        .and_then(|inventory| inventory.entries.first())
        .copied()
        .ok_or("depot: need a real starter backpack leaf to deposit")?;
    let original = inventory_units(client)?;
    let mut deposited = original.clone();
    let definition = leaf.item_definition_ref.get();
    let carried = deposited
        .get_mut(&definition)
        .ok_or("depot source absent from inventory")?;
    *carried = carried
        .checked_sub(u64::from(leaf.count.get()))
        .ok_or("depot source underflow")?;
    if *carried == 0 {
        deposited.remove(&definition);
    }
    let moved = super::full_row_e08::depot_intent(
        client,
        depot::DepotIntent {
            town: browse.town,
            box_number: 1,
            page: 0,
            action: depot::DepotAction::Deposit(leaf.handle),
        },
    )
    .await?;
    if moved.outcome != depot::DepotOutcome::Moved || moved.total != browse.total.saturating_add(1)
    {
        return Err(format!("depot: deposit did not add an entry: {moved:?}").into());
    }
    wait_inventory(client, &deposited).await?;
    let current = super::full_row_e08::depot_intent(
        client,
        depot::DepotIntent {
            town: browse.town,
            box_number: 1,
            page: 0,
            action: depot::DepotAction::Browse,
        },
    )
    .await?;
    let stored = current
        .entries
        .first()
        .filter(|entry| entry.definition == definition && entry.count == leaf.count.get())
        .ok_or("depot: newly deposited entry absent from refreshed box")?;
    let restored = super::full_row_e08::depot_intent(
        client,
        depot::DepotIntent {
            town: browse.town,
            box_number: 1,
            page: 0,
            action: depot::DepotAction::Withdraw(stored.ordinal),
        },
    )
    .await?;
    if restored.outcome != depot::DepotOutcome::Moved || restored.total != browse.total {
        return Err(
            format!("depot: withdrawal did not remove deposited entry: {restored:?}").into(),
        );
    }
    wait_inventory(client, &original).await?;
    row.pass("depot", "exact-deposit-and-withdrawal-custody");
    let mut panel = ui::StashPanel {
        selected: true,
        ..Default::default()
    };
    let command = ui::request(&mut panel, ui::Intent::Refresh).ok_or("stash browse input")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    if reply.reply.outcome != ui::stash::StashOutcome::Opened {
        return Err(format!("stash open: {:?}", reply.reply.outcome).into());
    }
    let item = client
        .inventory()
        .and_then(|i| i.entries.iter().find(|i| i.count.get() >= 2))
        .copied()
        .ok_or("no carried stack of at least two units for chosen-count withdrawal")?;
    let carried = inventory_units(client)?;
    let before: u64 = panel.rows.iter().map(|r| u64::from(r.quantity)).sum();
    let command =
        ui::request(&mut panel, ui::Intent::Stow(item.handle)).ok_or("stash stow input")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    if reply.reply.outcome != ui::stash::StashOutcome::Committed
        || reply.reply.stowed_quantity != item.count.get()
    {
        return Err(format!("stash stow: {:?}", reply.reply).into());
    }
    let command = ui::request(&mut panel, ui::Intent::Refresh).ok_or("stash refresh after stow")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    let after: u64 = panel.rows.iter().map(|r| u64::from(r.quantity)).sum();
    if after != before + u64::from(item.count.get()) {
        return Err("stash count delta differs from stowed quantity".into());
    }
    row.pass("stow", "stash-total-increased-by-exact-stack");
    let definitions = provisioning::item_index()?;
    let selected = panel
        .rows
        .iter()
        .position(|r| {
            definitions.definition_ref("Item", &r.definition_key, &r.definition_revision)
                == Some(item.item_definition_ref)
        })
        .ok_or("stowed definition absent from stash page")?;
    let command = ui::request(
        &mut panel,
        ui::Intent::Withdraw {
            row: selected,
            amount: 1,
        },
    )
    .ok_or("chosen-count withdrawal input")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    if reply.reply.outcome != ui::stash::StashOutcome::Committed {
        return Err(format!("withdraw refused: {:?}", reply.reply.outcome).into());
    }
    let command =
        ui::request(&mut panel, ui::Intent::Refresh).ok_or("stash refresh after withdraw")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    if panel
        .rows
        .iter()
        .map(|r| u64::from(r.quantity))
        .sum::<u64>()
        != after - 1
    {
        return Err("chosen one-unit withdrawal changed the wrong stash quantity".into());
    }
    let mut expected = carried;
    let units = expected
        .get_mut(&item.item_definition_ref.get())
        .ok_or("stowed inventory definition absent")?;
    *units = units
        .checked_sub(u64::from(item.count.get()) - 1)
        .ok_or("stow inventory underflow")?;
    wait_inventory(client, &expected).await?;
    row.pass(
        "chosen-withdraw",
        "one-unit-inventory-increase-and-one-unit-stash-decrease",
    );
    let stash_before: BTreeMap<_, _> = panel
        .rows
        .iter()
        .map(|r| (r.definition_key.clone(), r.quantity))
        .collect();
    relog(client, clients).await?;
    if inventory_units(client)? != expected {
        return Err("stash relog inventory changed".into());
    }
    let command =
        ui::request(&mut panel, ui::Intent::Refresh).ok_or("fresh-session stash browse")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    let stash_after: BTreeMap<_, _> = panel
        .rows
        .iter()
        .map(|r| (r.definition_key.clone(), r.quantity))
        .collect();
    if reply.reply.outcome != ui::stash::StashOutcome::Opened || stash_after != stash_before {
        return Err("fresh-session stash differs from committed withdrawal".into());
    }
    evidence(
        "E2EF4_OBSERVATION E07 Game verifier probe retained exact inventory and stash quantities after reconnect",
    );
    evidence(&format!("E2EF4_GAP E07 {}", platform_admission_gap()));
    Err(
        "inbox: need delivered mail or market item and retrieve from real Inbox search results; source:crates/protocol-oteryn/src/depot_search.rs:51"
            .into(),
    )
}

async fn party(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    rat_xp: i64,
) -> TestResult {
    use oteryn_dev_client::client_runtime::party as runtime;
    use runtime::party::{PartyDisposition, PartyIntent};
    async fn send(
        client: &mut DevClientSession,
        intent: Option<PartyIntent>,
    ) -> TestResult<runtime::party::PartyResult> {
        let command = match intent {
            Some(intent) => {
                runtime::PartyCommand::intent(intent).map_err(|e| format!("party input: {e:?}"))?
            }
            None => runtime::PartyCommand::query(),
        };
        let reply = runtime::send(client.session_mut(), &command).await?;
        let operation = runtime::party::decode_party_result(&reply.operation)
            .map_err(|e| format!("party operation: {e:?}"))?;
        if operation.disposition != PartyDisposition::Ok {
            return Err(format!("party refused: {:?}", operation.disposition).into());
        }
        runtime::party::decode_party_result(
            reply.view.as_deref().ok_or("party current view absent")?,
        )
        .map_err(|e| format!("party view: {e:?}").into())
    }
    let mut peer = keep_alive(client, connect_character(clients, 0x82, 0)).await?;
    let result: TestResult = async {
        let position = tile(peer.world_spatial().actor_position)?;
        keep_alive(&mut peer, approach_position(client, position, world)).await?;
        send(client, Some(PartyIntent::Invite(own(&peer)?))).await?;
        let invitations = send(&mut peer, None).await?;
        let handle = invitations
            .invitations
            .first()
            .ok_or("party invitation not visible to peer")?
            .handle;
        row.pass("invite", "peer-current-query-has-invitation");
        let state = send(&mut peer, Some(PartyIntent::Accept(handle))).await?;
        if state.members.len() != 2 {
            return Err("party join did not produce two members".into());
        }
        row.pass("join", "two-current-members");
        let result = client
            .chat(&oteryn_dev_client::ChatIntent::Say {
                mode: oteryn_dev_client::ChatSpeechMode::Say,
                text: "/sharedxp on".into(),
            })
            .await?;
        if result.disposition != oteryn_dev_client::ChatDisposition::Ok
            || !send(client, None).await?.shared_xp_enabled
        {
            return Err("shared XP toggle not reflected by party owner".into());
        }
        row.pass(
            "shared-xp-toggle",
            "leader-chat-toggle-current-state-enabled",
        );
        let state = send(client, None).await?;
        let member = state
            .members
            .iter()
            .find(|m| !m.leader)
            .ok_or("no nonleader member")?;
        let target_name = member.name.clone();
        let state = send(client, Some(PartyIntent::PassLeadership(member.handle))).await?;
        if !state
            .members
            .iter()
            .any(|m| m.leader && m.name == target_name)
        {
            return Err("leadership did not pass to selected member".into());
        }
        row.pass("leadership", "current-query-selected-member-is-leader");
        let split: TestResult = async {
            const RAT: &str = "oteryn:creature.rat";
            let first_vocation = keep_alive(&mut peer, party_base_vocation(client)).await?;
            let second_vocation = keep_alive(client, party_base_vocation(&mut peer)).await?;
            let boost = keep_alive(&mut peer, client_daily_boost::reward_selection(client)).await?;
            let base = i64::try_from(client_daily_boost::boosted_xp(u64::try_from(rat_xp)?, RAT, &boost)?)?;
            let expected = two_member_share(base, [first_vocation, second_vocation])?;
            keep_alive(&mut peer, travel_to_spawn(client, world, spawns, RAT)).await?;
            let target = adjacent_creature(client)
                .ok_or("shared-xp-split: authored rat route produced no adjacent live creature")?;
            let creature = client.world_entities().and_then(|view| view.get(&target))
                .ok_or("shared-xp-split: visible rat disappeared before peer approach")?;
            let rat = tile(creature.position)?;
            let first = tile(client.world_spatial().actor_position)?;
            let path = route(world, tile(peer.world_spatial().actor_position)?, |at| {
                at != first && at != rat && at.floor == rat.floor
                    && at.x.abs_diff(rat.x).max(at.y.abs_diff(rat.y)) <= 1
                    && world.base().tile(at.x, at.y, at.floor).is_some_and(|tile| !tile.protection_zone())
            }).ok_or("shared-xp-split: no reachable second attack tile beside the live rat")?;
            keep_alive(client, walk(&mut peer, world, &path)).await?;
            let attack = peer.session_mut().attack_target(Some(&target)).await?;
            if attack.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
                return Err(format!("shared-xp-split: peer contribution refused: {attack:?}").into());
            }
            let attack = client.session_mut().attack_target(Some(&target)).await?;
            if attack.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
                return Err(format!("shared-xp-split: main contribution refused: {attack:?}").into());
            }
            let death = keep_alive(&mut peer, committed_hunt_death(client, clients, None, RAT, target,
                tokio::time::Instant::now() + Duration::from_secs(45))).await?;
            let first = death.awards.get(&clients.characters[0]).copied();
            let second = death.awards.get(&clients.characters[1]).copied();
            if !death.shared || death.awards.len() != 2 || first.is_none() || second.is_none() {
                return Err(format!("shared-xp-split: exact kill settled with shared={} awards={:?}; need positive awards for both eligible members; source:crates/gameplay/durability/src/durability/party_kill_xp.rs:1", death.shared, death.awards).into());
            }
            let occurrence = death.reward_occurrence.ok_or("shared-xp-split: principal reward occurrence absent")?;
            let mut database = sqlx::PgConnection::connect(clients.url).await?;
            let observed: Vec<(i64, i64)> = sqlx::query_as(
                "SELECT r.base_xp, m.raw_share FROM game_party_kill_xp_members p
                 JOIN game_party_kill_xp_members m USING (death_key)
                 JOIN game_party_kill_xp_receipts r USING (death_key)
                 WHERE p.reward_occurrence_id=encode($1,'hex')::uuid LIMIT 3",
            ).bind(occurrence.as_slice()).fetch_all(&mut database).await?;
            let raw: Vec<_> = observed.iter().map(|&(_, share)| share).collect();
            if observed.iter().any(|&(receipt_base, _)| receipt_base != base)
                || !exact_party_share(&raw, expected)
            {
                return Err(format!("shared-xp-split: expected reference kill base={base} and two raw shares of {expected}; observed={observed:?}").into());
            }
            row.pass("shared-xp-split", format!("exact-rat-death-receipt-base={base}-both-raw-shares={expected}"));
            Ok(())
        }.await;
        send(client, Some(PartyIntent::Leave)).await?;
        if !send(client, None).await?.members.is_empty() {
            return Err("party leave retained membership".into());
        }
        row.pass("leave", "leaver-current-query-empty");
        split
    }
    .await;
    if result.is_err() {
        let _ = send(client, Some(PartyIntent::Leave)).await;
    }
    keep_alive(client, return_to_pz(&mut peer, world)).await?;
    let (cleanup_result, cleanup, attempts, busy_timed_out) =
        keep_alive(client, super::logout_retrying_busy(&mut peer)).await?;
    if cleanup_result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("party peer cleanup logout refused: {cleanup}; attempts={attempts}; busy_timed_out={busy_timed_out}; journey={result:?}").into());
    }
    result
}

async fn guild(row: &mut Row, client: &mut DevClientSession) -> TestResult {
    use oteryn_dev_client::client_runtime::guild as runtime;
    let reply = runtime::send(
        client.session_mut(),
        &runtime::GuildCommand::Query(runtime::guild::GuildQuery::Own),
    )
    .await?;
    // Only an answered query reaches the missing MOTD route; a failed one stays FAIL.
    row.missing_server = verdict::guild_motd_missing(&reply);
    // Native GuildIntent has membership/rank writers but no MOTD writer. Reading
    // GuildSummary.message cannot prove an update. Keep every other gap visible too.
    evidence(
        "E2EF4_MISSING S03 motd source:crates/protocol-oteryn/src/guild.rs:17 native-GuildIntent-has-no-MOTD-write",
    );
    Err(format!("guild-query={reply:?};guild/ranks/chat/bank/wars/relog assertions unfinished;MOTD-write-unavailable").into())
}

async fn logout(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult {
    keep_alive(client, logout_peer(row, clients, world, spawns)).await
}

async fn logout_peer(
    row: &mut Row,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult {
    let mut peer = connect_character(clients, 0x83, 0).await?;
    return_to_pz(&mut peer, world).await?;
    let position = peer.world_spatial().actor_position;
    if !world
        .base()
        .tile(tile(position)?.x, tile(position)?.y, tile(position)?.floor)
        .is_some_and(|t| t.protection_zone())
    {
        let (_, cleanup, attempts, busy_timed_out) = super::logout_retrying_busy(&mut peer).await?;
        return Err(format!("logout peer is not on authored PZ tile: cleanup={cleanup}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
    }
    let session = peer.join_snapshot().game_session_id;
    let (result, logout, attempts, busy_timed_out) = super::logout_retrying_busy(&mut peer).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!(
            "PZ logout refused: {logout}; attempts={attempts}; busy_timed_out={busy_timed_out}"
        )
        .into());
    }
    row.pass("pz-logout", "authored-PZ-native-logout-accepted");
    drop(peer);
    let mut peer = connect_character(clients, 0x84, 0).await?;
    if peer.world_spatial().actor_position != position
        || peer.join_snapshot().game_session_id == session
    {
        return Err("fresh-login-position: saved PZ tile or session mismatch".into());
    }
    evidence(
        "E2EF4_OBSERVATION S09 Game verifier probe retained the saved PZ position after reconnect",
    );
    evidence(&format!("E2EF4_GAP S09 {}", platform_admission_gap()));

    let probes: TestResult = async {
        let from = tile(peer.world_spatial().actor_position)?;
        let no_logout = route(world, from, |at| {
            world.base().tile(at.x, at.y, at.floor).is_some_and(|tile| tile.no_logout())
        });
        if let Some(path) = no_logout {
            walk(&mut peer, world, &path).await?;
            let response = peer.session_mut().logout().await?;
            if response.status != CommandStatus::Rejected || response.result.is_some() {
                return Err(format!("nologout: expected rejection with no result, observed {response:?}").into());
            }
            peer.service_liveness(Duration::from_millis(25)).await?;
            row.pass("nologout", "authored-no-logout-rejected-and-session-live");
        } else {
            evidence("E2EF4_GAP S09 nologout: no reachable authored NOLOGOUT tile; source:crates/gameplay/dispatch/src/gameplay_transport/logout.rs:153");
        }
        travel_to_spawn(&mut peer, world, spawns, "oteryn:creature.rat").await?;
        if adjacent_creature(&peer).is_none() {
            return Err("fight-refusal: no adjacent live creature at authored rat route".into());
        }
        client_hunt::hunt(&mut peer).await?;
        let response = peer.session_mut().logout().await?;
        if !matches!(response.result, Some(oteryn_protocol_oteryn::logout::LogoutResult::InFight { retry_after_ms }) if retry_after_ms > 0) {
            return Err(format!("fight-refusal: expected InFight after real kill, observed {response:?}").into());
        }
        row.pass("fight-refusal", "real-combat-logout-in-fight-with-retry");
        Ok(())
    }.await;
    if let Err(error) = &probes {
        evidence(&format!("E2EF4_GAP S09 {error}"));
        // An unexpected accepted logout already released this session.
        return probes;
    }
    return_to_pz(&mut peer, world).await?;
    let (result, reply, attempts, timeout) = logout_retrying_busy(&mut peer).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!(
            "logout cleanup: {reply}; attempts={attempts}; busy_timed_out={timeout}"
        )
        .into());
    }
    Err("idle-warning/idle-removal/gameplay-resets-idle: need real 15/16-minute inactivity and a separate gameplay reset observation; source:crates/gameplay/dispatch/src/gameplay_transport/connection/serve/idle.rs:32".into())
}

fn adjacent_creature(client: &DevClientSession) -> Option<EntityRef> {
    let here = client.world_spatial().actor_position;
    client
        .world_entities()?
        .iter()
        .find(|entry| {
            entry.kind == oteryn_dev_client::EntityKind::Creature
                && entry.position.floor == here.floor
                && entry
                    .position
                    .x
                    .abs_diff(here.x)
                    .max(entry.position.y.abs_diff(here.y))
                    <= 1
        })
        .map(|entry| entry.entity)
}

async fn return_to_pz(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| tile.protection_zone() && !tile.no_logout())
    })
    .ok_or("no reachable authored logout-capable PZ")?;
    walk(client, world, &path).await
}

async fn speech(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    evidence(&format!("E2EF4_GAP S18 {}", platform_admission_gap()));
    use oteryn_dev_client::{ChatDisposition, ChatIntent, ChatSpeechMode};
    let mut peer = keep_alive(client, connect_character(clients, 0x86, 0)).await?;
    let journey: TestResult = async {
        let position = tile(peer.world_spatial().actor_position)?;
        keep_alive(&mut peer, approach_position(client, position, world)).await?;
        let speaker = own(client)?;
        for (action, mode, text) in [
            ("say", ChatSpeechMode::Say, "E2EF4 adjacent say"),
            ("whisper", ChatSpeechMode::Whisper, "E2EF4 adjacent whisper"),
        ] {
            let line = super::client_row_s18::speak(client, mode, text).await?;
            if !matches!(line, oteryn_dev_client::ChatLine::Local { speaker: actual, .. }
                if actual.identity == speaker.identity && actual.generation.get() == speaker.generation)
            {
                return Err(format!("{action}: wrong sender on echo").into());
            }
            wait_heard(&mut peer, speaker, mode, text, true).await?;
            row.pass(action, "exact-local-echo-and-adjacent-peer-line");
        }

        let speaker_at = tile(client.world_spatial().actor_position)?;
        let path = route(world, tile(peer.world_spatial().actor_position)?, |at| {
            let dx = at.x.abs_diff(speaker_at.x);
            let dy = at.y.abs_diff(speaker_at.y);
            at.floor == speaker_at.floor && dx <= 8 && dy <= 6 && (dx == 8 || dy == 6)
        }).ok_or("range: no walkable same-floor tile on the say/whisper boundary")?;
        keep_alive(client, walk(&mut peer, world, &path)).await?;
        super::client_row_s18::speak(client, ChatSpeechMode::Say, "E2EF4 boundary say").await?;
        wait_heard(&mut peer, speaker, ChatSpeechMode::Say, "E2EF4 boundary say", true).await?;
        super::client_row_s18::speak(client, ChatSpeechMode::Whisper, "E2EF4 private text").await?;
        wait_heard(&mut peer, speaker, ChatSpeechMode::Whisper, "pspsps", true).await?;
        if heard(&peer, speaker, ChatSpeechMode::Whisper, "E2EF4 private text") {
            return Err("range: distant whisper leaked its clear text".into());
        }
        let path = route(world, tile(peer.world_spatial().actor_position)?, |at| {
            let dx = at.x.abs_diff(speaker_at.x);
            let dy = at.y.abs_diff(speaker_at.y);
            at.floor == speaker_at.floor && ((dx == 9 && dy <= 6) || (dy == 7 && dx <= 8))
        }).ok_or("range: no walkable same-floor tile immediately outside the say boundary")?;
        keep_alive(client, walk(&mut peer, world, &path)).await?;
        super::client_row_s18::speak(client, ChatSpeechMode::Say, "E2EF4 outside say").await?;
        wait_heard(&mut peer, speaker, ChatSpeechMode::Say, "E2EF4 outside say", false).await?;
        row.pass("range", "say-boundary-inside-outside-and-obscured-whisper");

        let yell = client.chat(&ChatIntent::Say {
            mode: ChatSpeechMode::Yell, text: "E2EF4 yell".into(),
        }).await?;
        match yell.disposition {
            ChatDisposition::Ok => {
                wait_heard(client, speaker, ChatSpeechMode::Yell, "E2EF4 YELL", true).await?;
                wait_heard(&mut peer, speaker, ChatSpeechMode::Yell, "E2EF4 YELL", true).await?;
                row.pass("yell", "uppercase-local-echo-and-peer-line-beyond-say-range");
            }
            ChatDisposition::LevelTooLow => evidence("E2EF4_OBSERVATION S18 yell level-too-low"),
            other => return Err(format!("yell: unexpected disposition {other:?}").into()),
        }
        Ok(())
    }.await;
    let (cleanup, reply, attempts, timeout) = logout_retrying_busy(&mut peer).await?;
    if cleanup != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("speech peer logout: {reply}; attempts={attempts}; busy_timed_out={timeout}; journey={journey:?}").into());
    }
    journey?;
    for index in 0..32 {
        let result = client
            .chat(&ChatIntent::Say {
                mode: ChatSpeechMode::Say,
                text: format!("E2EF4 mute {index}"),
            })
            .await?;
        if result.disposition == ChatDisposition::Muted && result.wait_seconds > 0 {
            row.pass("mute", "native-spam-gate-muted-with-positive-wait");
            break;
        }
    }
    evidence(
        "E2EF4_GAP S18 yell-level-premium: need level-eligible yell and both under-level premium states; source:crates/gameplay/dispatch/src/gameplay_transport/chat_intent.rs:837",
    );
    Err(platform_admission_gap())
}

fn heard(
    client: &DevClientSession,
    sender: EntityRef,
    expected_mode: oteryn_dev_client::ChatSpeechMode,
    expected_text: &str,
) -> bool {
    client.chat_log().is_some_and(|log| {
        log.lines().any(|line| matches!(
        line, oteryn_dev_client::ChatLine::Local { speaker, mode, text, .. }
            if speaker.identity == sender.identity && speaker.generation.get() == sender.generation
                && *mode == expected_mode && text == expected_text
    ))
    })
}

async fn wait_heard(
    client: &mut DevClientSession,
    sender: EntityRef,
    mode: oteryn_dev_client::ChatSpeechMode,
    text: &str,
    expected: bool,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        if heard(client, sender, mode, text) {
            return if expected {
                Ok(())
            } else {
                Err(format!("range: peer unexpectedly heard {text:?}").into())
            };
        }
        if tokio::time::Instant::now() >= deadline {
            return if expected {
                Err(format!("range: peer did not hear {text:?}").into())
            } else {
                Ok(())
            };
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speech_requires_fresh_login_evidence() {
        let mut row = Row::new("E2EF4", "S18", S18);
        for action in [
            "say",
            "whisper",
            "yell",
            "range",
            "yell-level-premium",
            "mute",
        ] {
            row.pass(action, "observed");
        }
        assert!(
            !row.complete(),
            "speech without fresh admission is incomplete"
        );
    }

    #[test]
    fn a_buy_and_ordinary_sale_cannot_qualify_shop() {
        let mut row = Row::new("E2EF4", "E01", E01);
        for action in [
            "greet",
            "look",
            "buy",
            "sell",
            "deposit",
            "withdraw",
            "transfer",
            "relog-inventory-bank",
        ] {
            row.pass(action, "observed");
        }
        assert!(!row.complete());
        row.pass("sell-all", "observed");
        assert!(row.complete());
    }

    #[test]
    fn denomination_plans_require_the_exact_source_and_preserve_value() {
        assert_eq!(exchange_counts([99, 0, 0], COIN_STEPS[0]), None);
        assert_eq!(exchange_counts([0, 99, 0], COIN_STEPS[2]), None);
        assert_eq!(exchange_counts([100, u64::MAX, 0], COIN_STEPS[0]), None);
        for [a, b] in [
            [COIN_STEPS[0], COIN_STEPS[1]],
            [COIN_STEPS[2], COIN_STEPS[3]],
        ] {
            assert_eq!(
                exchange_counts(a.0, a).and_then(|count| exchange_counts(count, b)),
                Some(a.0)
            );
        }
    }

    #[test]
    fn an_offer_from_another_book_does_not_qualify_settlement() {
        use oteryn_protocol_oteryn::market::{MarketReply, MarketRow};
        let offer = [1; 16];
        let mut reply = MarketReply {
            action: 4,
            code: 1,
            offer: [0; 16],
            rows: vec![MarketRow {
                offer,
                definition: 10,
                side: 1,
                amount: 1,
                price: 1,
                state: 2,
                anonymous: false,
            }],
        };
        assert!(settled_offer(&reply, offer, 10));
        assert!(!settled_offer(&reply, offer, 11));
        reply.rows[0].state = 1;
        assert!(!settled_offer(&reply, offer, 10));
    }
}

async fn while_servicing<T, P>(
    operation: impl Future<Output = TestResult<T>>,
    parked: &mut P,
    mut service: impl AsyncFnMut(&mut P) -> TestResult,
) -> TestResult<T> {
    let (done, mut stop) = tokio::sync::oneshot::channel();
    let (result, liveness) = tokio::join!(
        async {
            let result = operation.await;
            let _ = done.send(());
            result
        },
        async {
            while stop
                .try_recv()
                .is_err_and(|error| error == tokio::sync::oneshot::error::TryRecvError::Empty)
            {
                service(parked).await?;
            }
            Ok::<_, Box<dyn std::error::Error>>(())
        },
    );
    // A broken parked connection must reach the caller, even if the operation
    // also failed an assertion, so the family discards unusable sessions.
    liveness?;
    result
}

pub(super) async fn keep_alive<T>(
    parked: &mut DevClientSession,
    operation: impl Future<Output = TestResult<T>>,
) -> TestResult<T> {
    while_servicing(operation, parked, async |session| {
        session.service_liveness(Duration::from_millis(100)).await?;
        session.take_events();
        Ok(())
    })
    .await
}

pub(super) fn platform_admission_gap() -> Box<dyn std::error::Error> {
    "platform-admission: Platform-issued peer and fresh grants unavailable in the reference World harness; source:apps/game-server/src/qualification.rs:2720".into()
}

#[test]
fn parked_session_is_serviced_until_operation_completes() -> TestResult {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(async {
        let mut probes = 0;
        let (ready, response) = tokio::sync::oneshot::channel();
        let mut ready = Some(ready);
        let op = async {
            tokio::time::timeout(Duration::from_millis(100), response).await??;
            Ok(())
        };
        while_servicing(op, &mut probes, async |probes| {
            *probes += 1;
            if let Some(ready) = ready.take() {
                let _ = ready.send(());
            }
            tokio::task::yield_now().await;
            Ok(())
        })
        .await?;
        assert!(probes > 0);
        Ok(())
    })
}

async fn party_base_vocation(client: &mut DevClientSession) -> TestResult<u8> {
    use oteryn_protocol_oteryn::cyclopedia_character as wire;
    let request = [wire::CharacterTab::GeneralStats as u8, 0, 0, 1];
    let reply = client
        .session_mut()
        .gated_command(wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1, 388, &request)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err("shared-xp-split: GeneralStats vocation query refused".into());
    }
    match general_stats::validate(&reply.payload, general_stats::selected(client))? {
        Some(1) => Ok(0),
        Some(id @ 2..=11) => Ok((id - 2) / 2 + 1),
        _ => Err("shared-xp-split: native vocation unavailable".into()),
    }
}

// Independent reference oracle for two players contributing the entire rat pool,
// without summoned attackers, event rates or Animus mastery adjustments.
fn two_member_share(base: i64, [first, second]: [u8; 2]) -> TestResult<i64> {
    let bonus = if first != 0 && second != 0 && first != second {
        30
    } else if base >= 20 {
        20
    } else {
        0
    };
    let numerator = i128::from(base) * (100 + bonus);
    Ok(i64::try_from((numerator + 199) / 200)?)
}

fn exact_party_share(raw: &[i64], expected: i64) -> bool {
    expected > 0 && raw == [expected; 2]
}

async fn finish_row(client: &mut Option<DevClientSession>, row: Row, result: TestResult) {
    verify_primary(client, &result, async |primary| {
        if let Err(error) = primary.service_liveness(Duration::from_millis(25)).await {
            evidence(&format!("E2EF4_GAP primary-session {error}"));
            return Err(error.into());
        }
        Ok(())
    })
    .await;
    row.finish(&require_platform_admission(result));
}

async fn verify_primary<T>(
    slot: &mut Option<T>,
    result: &TestResult,
    probe: impl AsyncFnOnce(&mut T) -> TestResult,
) {
    if result.is_err()
        && let Some(primary) = slot.as_mut()
        && probe(primary).await.is_err()
    {
        *slot = None;
    }
}

fn require_platform_admission(result: TestResult) -> TestResult {
    result.and_then(|()| Err(platform_admission_gap()))
}

#[cfg(test)]
mod review_regressions {
    use super::*;

    #[test]
    fn review_equal_but_wrong_party_share_fails() -> TestResult {
        let plain_rat_share = two_member_share(5, [1, 1])?;
        assert_eq!(plain_rat_share, 3);
        assert!(!exact_party_share(&[4, 4], plain_rat_share));
        assert!(exact_party_share(&[3, 3], plain_rat_share));
        assert_eq!(two_member_share(5, [1, 2])?, 4);
        assert_eq!(two_member_share(20, [0, 1])?, 12);
        Ok(())
    }

    #[test]
    fn review_fixture_only_success_cannot_qualify_platform_admission() {
        assert!(require_platform_admission(Ok(())).is_err());
    }

    #[test]
    fn review_primary_probe_controls_reuse_after_transport_failure() -> TestResult {
        let rt = tokio::runtime::Builder::new_current_thread().build()?;
        rt.block_on(async {
            let transport = Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof).into());
            let mut slot = Some(());
            verify_primary(&mut slot, &transport, async |_| Ok(())).await;
            assert!(
                slot.is_some(),
                "a peer failure must leave the healthy primary available"
            );
            verify_primary(&mut slot, &transport, async |_| {
                Err("primary poisoned".into())
            })
            .await;
            assert!(
                slot.is_none(),
                "a failed primary probe must discard the session"
            );
            Ok(())
        })
    }
}
