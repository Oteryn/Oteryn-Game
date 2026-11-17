//! MP04: two Accounts trade on the World Market over the real MARKET_V1 wire (capability 63,
//! command 80) at authored depot lockers.
//!
//! A (core/0) deposits wares and sells from its depot, B (packets/0) browses and buys part,
//! A cancels the rest, B places a buy offer that A fills from its depot, and both read their
//! history. Bank gold is read over ATELIER_RESOURCES_V1 (PostgreSQL
//! `game_account_bank_balances`) and depot/Inbox units over Depot Search, so every assertion
//! reads server-owned state through the client wire. A refusal row passes only on its exact
//! refusal code with balance, depot and offers unchanged. No row passes without a reply.
use super::*;
use bots::Bot;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::npc_shop;
use oteryn_protocol_oteryn::item_view::ItemHandle;
use oteryn_protocol_oteryn::market::{self as wire, MarketReply, MarketRequest, MarketRow};
use oteryn_protocol_oteryn::{depot, depot_search as search};
use std::collections::BTreeMap;

const A: usize = 0;
const B: usize = 1;
const SELL_PRICE: u64 = 50;
const BUY_PRICE: u64 = 40;
/// MP01 equips A's axe and the core profile journey checks it again (provision.rs:56-67).
const AXE: u32 = 3276;
/// Open offers per Character: crates/gameplay/economy/src/market.rs:28 and
/// apps/game-server/migrations/0191_market_settlement.sql:311 (CipSoft manual, MARKET0 decision
/// docs/architecture/reviews/OTERYN_GAME_MARKET0_WORLD_MARKET_DECISION_2026-09-30.md:67-68).
/// Crystal 201189f8 `src/io/iomarket.hpp:33-34` bounds only the projected book.
const OFFER_CAP: usize = 100;
const ZERO: [u8; 16] = [0; 16];
const ROWS: [&str; 16] = [
    "locker",
    "open",
    "deposit",
    "sell_offer",
    "browse",
    "partial_purchase",
    "settlement",
    "cancel_remainder",
    "buy_offer",
    "sell_into_buy",
    "history_both_sides",
    "excess_items_refused",
    "insufficient_gold_refused",
    "offer_cap_refused",
    "relog_persistence",
    "cleanup",
];

/// Placement fee per Tibia news 8996: 2.5%, at least 20 and at most 2,500,000 gold
/// (apps/game-server/migrations/0296_market_fee_2026_10.sql:30). Crystal 201189f8
/// `src/game/game.cpp:10502-10504` still charges 2% capped at 1,000,000; that divergence is
/// pinned in crates/gameplay/economy/src/market_crystal_parity_tests.rs:51-58.
fn fee(total: u64) -> u64 {
    (total * 25 / 1000).clamp(20, 2_500_000)
}

#[derive(Debug, Clone, Copy)]
struct Ware {
    client_id: u32,
    definition: u32,
}

#[derive(Default)]
struct State {
    towns: [Option<u16>; 2],
    wares: Option<(Ware, Ware)>,
    sell: Option<[u8; 16]>,
    bought: bool,
    cancelled: bool,
    buy: Option<[u8; 16]>,
    sold: bool,
    /// Every offer the server accepted and nothing has closed yet, by bot index.
    open: Vec<(usize, [u8; 16])>,
}

struct Before {
    banks: [u64; 2],
    inbox: u32,
}

fn check(step: &str, result: TestResult<String>, failures: &mut Vec<String>) -> bool {
    match result {
        Ok(detail) => {
            evidence(&format!("MP_CHECK MP04 {step} PASS {detail}"));
            true
        }
        Err(error) => {
            verdict::record_assertion(step);
            evidence(&format!(
                "MP_CHECK MP04 {step} FAIL {error};source:{}:{}",
                file!(),
                line!()
            ));
            failures.push(format!("{step}:{error}"));
            false
        }
    }
}

fn blocked(step: &str, by: &str, failures: &mut Vec<String>) {
    check(step, Err(format!("blocked by {by}").into()), failures);
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
        .map_err(|_| format!("MP04 {phase} deadline;source:{}:{}", file!(), line!()))?
}

/// The server's meaning of a Market reply code, with the line that produces it.
fn explain(code: u8) -> &'static str {
    match code {
        1 => "ok",
        2 => {
            "insufficient bank gold;source:apps/game-server/migrations/0191_market_settlement.sql:386"
        }
        3 => {
            "items not in this town's depot;source:apps/game-server/migrations/0191_market_settlement.sql:373"
        }
        4 => "Inbox full;source:apps/game-server/migrations/0191_market_settlement.sql:379",
        5 => "open-offer cap;source:apps/game-server/migrations/0191_market_settlement.sql:311",
        6 => "offer gone;source:crates/gameplay/dispatch/src/gameplay_transport/market.rs:190",
        7 => "same Account;source:apps/game-server/migrations/0191_market_settlement.sql:332",
        8 => "touched-item limit;source:apps/game-server/migrations/0191_market_settlement.sql:369",
        9 => {
            "rejected by fence/capability/advance;source:crates/gameplay/dispatch/src/gameplay_transport/market.rs:127"
        }
        10 => "balance limit;source:apps/game-server/migrations/0191_market_settlement.sql:389",
        11 => {
            "no depot locker;source:crates/gameplay/dispatch/src/gameplay_transport/market.rs:151"
        }
        12 => {
            "SERVER_GAP Premium gate closed before Premium delivery, contrary to MARKET0 owner \
             answer Q3 (docs/architecture/reviews/OTERYN_GAME_MARKET0_WORLD_MARKET_DECISION_2026-09-30.md:135-136,387-389);\
             source:crates/gameplay/dispatch/src/gameplay_transport/market.rs:163-170,\
             crates/gameplay/dispatch/src/gameplay_transport/spell_premium_coordinator.rs:540-541"
        }
        13 => {
            "junior Character;source:crates/gameplay/dispatch/src/gameplay_transport/market.rs:172"
        }
        14 => "not marketable;source:crates/gameplay/dispatch/src/gameplay_transport/market.rs:158",
        15 => "book busy;source:apps/game-server/migrations/0191_market_settlement.sql:333",
        _ => "unknown code",
    }
}

fn hex(id: &[u8; 16]) -> String {
    id.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn request(action: u8, definition: u32, offer: [u8; 16], amount: u32, price: u64) -> MarketRequest {
    MarketRequest {
        action,
        definition,
        offer,
        amount,
        price,
        anonymous: false,
        page: 0,
    }
}

async fn market(bot: &mut Bot<'_, '_>, request: MarketRequest) -> TestResult<MarketReply> {
    let payload = wire::encode_request(request)
        .map_err(|error| format!("market encode {request:?}:{error:?}"))?;
    let response = bot
        .client()?
        .session_mut()
        .gated_command(
            wire::CAPABILITY_MARKET_V1,
            wire::COMMAND_TYPE_MARKET,
            &payload,
        )
        .await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!(
            "{} market action={} transport status {:?};source:crates/gameplay/dispatch/src/gameplay_transport/market.rs:113",
            bot.name, request.action, response.status
        )
        .into());
    }
    wire::decode_reply(&response.payload)
        .map_err(|error| format!("{} market decode {error:?}", bot.name).into())
}

/// Place an offer; an accepted one is tracked so cleanup cancels it whatever happens next.
async fn create(
    bots: &mut [Bot<'_, '_>],
    index: usize,
    request: MarketRequest,
    state: &mut State,
) -> TestResult<MarketReply> {
    let reply = market(&mut bots[index], request).await?;
    if reply.code == 1 {
        state.open.push((index, reply.offer));
    }
    Ok(reply)
}

async fn cancel(
    bots: &mut [Bot<'_, '_>],
    index: usize,
    offer: [u8; 16],
    state: &mut State,
) -> TestResult<MarketReply> {
    let reply = market(&mut bots[index], request(7, 0, offer, 0, 0)).await?;
    if matches!(reply.code, 1 | 6) {
        state
            .open
            .retain(|&(owner, open)| owner != index || open != offer);
    }
    Ok(reply)
}

async fn query(
    bot: &mut Bot<'_, '_>,
    action: u8,
    definition: u32,
    last_page: u32,
) -> TestResult<Vec<MarketRow>> {
    let mut rows = Vec::new();
    for page in 0..=last_page {
        let reply = market(
            bot,
            MarketRequest {
                page,
                ..request(action, definition, ZERO, 0, 0)
            },
        )
        .await?;
        if reply.code != 1 || reply.action != action {
            return Err(format!(
                "{} market query action={action} page={page} replied action={} code={} {}",
                bot.name,
                reply.action,
                reply.code,
                explain(reply.code)
            )
            .into());
        }
        let more = reply.rows.len() == 32;
        rows.extend(reply.rows);
        if !more {
            break;
        }
    }
    Ok(rows)
}

/// My Offers lists only open offers (crates/gameplay/durability/src/durability/market/settlement.rs:158).
async fn own_offers(bot: &mut Bot<'_, '_>) -> TestResult<Vec<MarketRow>> {
    query(bot, 3, 0, 62).await
}

async fn history(bot: &mut Bot<'_, '_>) -> TestResult<Vec<MarketRow>> {
    query(bot, 4, 0, 0).await
}

async fn bank(bot: &mut Bot<'_, '_>) -> TestResult<u64> {
    let balances = npc_shop::balances(bot.client()?.session_mut()).await?;
    balances.map(|balances| balances.bank_gold).ok_or_else(|| {
        format!(
            "{} bank view unavailable;source:crates/client-runtime/src/features/0081-npc-shop/protocol.rs:99",
            bot.name
        )
        .into()
    })
}

async fn search_command(
    bot: &mut Bot<'_, '_>,
    request: search::Request,
) -> TestResult<search::Reply> {
    let payload = search::encode_request(request)
        .map_err(|error| format!("depot search encode {error:?}"))?;
    let response = bot
        .client()?
        .session_mut()
        .gated_command(
            search::CAPABILITY_DEPOT_SEARCH_V1,
            search::COMMAND_TYPE_DEPOT_SEARCH,
            &payload,
        )
        .await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!("{} depot search status {:?}", bot.name, response.status).into());
    }
    search::decode_reply(&response.payload)
        .map_err(|error| format!("{} depot search decode {error:?}", bot.name).into())
}

/// Server-owned (depot, Inbox) units of one native item for this Character.
async fn holdings(bot: &mut Bot<'_, '_>, client_id: u32) -> TestResult<(u32, u32)> {
    let reply = search_command(bot, search::Request::Search { client_id }).await?;
    match reply.outcome {
        search::Outcome::NotFound => Ok((0, 0)),
        search::Outcome::Found => Ok(reply
            .results
            .iter()
            .find(|row| row.client_id == client_id)
            .map_or((0, 0), |row| (row.depot, row.inbox))),
        _ => Err(format!(
            "{} depot search {client_id} outcome {:?};source:crates/gameplay/dispatch/src/gameplay_transport/depot_search.rs:59",
            bot.name, reply.outcome
        )
        .into()),
    }
}

async fn depot_intent(
    bot: &mut Bot<'_, '_>,
    intent: depot::DepotIntent,
) -> TestResult<depot::DepotReply> {
    let payload =
        depot::encode_depot_intent(intent).map_err(|error| format!("depot encode {error:?}"))?;
    let response = bot
        .client()?
        .session_mut()
        .gated_command(
            depot::CAPABILITY_DEPOT_V1,
            depot::COMMAND_TYPE_DEPOT_INTENT,
            &payload,
        )
        .await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!("{} depot status {:?}", bot.name, response.status).into());
    }
    depot::decode_depot_reply(&response.payload)
        .map_err(|error| format!("{} depot decode {error:?}", bot.name).into())
}

fn holds_handle(client: &DevClientSession, handle: ItemHandle) -> bool {
    client.inventory().is_some_and(|inventory| {
        inventory
            .main_backpack
            .iter()
            .chain(&inventory.entries)
            .chain(inventory.equipment.iter().map(|entry| &entry.item))
            .any(|entry| entry.handle == handle)
    })
}

/// Walk to a free tile beside an authored locker and open box 1 there; the town comes from the
/// server's reply.
async fn reach_locker(
    bots: &mut [Bot<'_, '_>],
    index: usize,
    world: &BundleWorld,
) -> TestResult<u16> {
    let from = tile(bots[index].client()?.world_spatial().actor_position)?;
    let others = bots
        .iter()
        .enumerate()
        .filter(|(other, _)| *other != index)
        .map(|(_, bot)| {
            tile(
                bot.session
                    .as_ref()
                    .ok_or("MP04 peer session closed")?
                    .world_spatial()
                    .actor_position,
            )
        })
        .collect::<TestResult<Vec<_>>>()?;
    let path = route(world, from, |at| {
        near_locker(world, at) && !others.contains(&at)
    })
    .ok_or_else(|| {
        format!(
            "{} has no reachable depot locker tile;source:apps/game-server/src/qualification/client_phases/world.rs:522",
            bots[index].name
        )
    })?;
    bots::walk_bot(bots, index, world, &path).await?;
    let opened = depot_intent(
        &mut bots[index],
        depot::DepotIntent {
            town: None,
            box_number: 1,
            page: 0,
            action: depot::DepotAction::Browse,
        },
    )
    .await?;
    match opened.town {
        Some(town) if opened.outcome == depot::DepotOutcome::Opened => Ok(town),
        town => Err(format!(
            "{} depot browse replied {:?} town={town:?};source:crates/gameplay/dispatch/src/gameplay_transport/depot.rs",
            bots[index].name, opened.outcome
        )
        .into()),
    }
}

async fn lockers(
    bots: &mut [Bot<'_, '_>],
    world: &BundleWorld,
    state: &mut State,
) -> TestResult<String> {
    let a = reach_locker(bots, A, world).await?;
    state.towns[A] = Some(a);
    let b = reach_locker(bots, B, world).await?;
    state.towns[B] = Some(b);
    Ok(format!("A_town={a} B_town={b} depot_box=1 opened"))
}

async fn open(bots: &mut [Bot<'_, '_>]) -> TestResult<String> {
    let mut wares = Vec::new();
    for index in [A, B] {
        let reply = market(&mut bots[index], request(1, 0, ZERO, 0, 0)).await?;
        if reply.code != 1 || reply.action != 1 || reply.rows.is_empty() {
            return Err(format!(
                "{} wares replied action={} code={} rows={} {}",
                bots[index].name,
                reply.action,
                reply.code,
                reply.rows.len(),
                explain(reply.code)
            )
            .into());
        }
        wares.push(reply.rows.len());
    }
    Ok(format!("wares_page0 A={} B={}", wares[0], wares[1]))
}

/// Choose two marketable wares from A's backpack (the server's book reply decides
/// marketability) and deposit two units of X and one of Y at A's locker. Each needs two
/// units so A keeps one of every tool after the trades.
async fn stock(bots: &mut [Bot<'_, '_>], state: &mut State) -> TestResult<String> {
    let town = state.towns[A].ok_or("A town unknown")?;
    let definitions = provisioning::item_index()?;
    let entries = bots[A]
        .client()?
        .inventory()
        .map(|inventory| inventory.entries.clone())
        .unwrap_or_default();
    let mut carried: BTreeMap<u32, (u32, u32, Vec<(ItemHandle, u32)>)> = BTreeMap::new();
    for entry in entries {
        let Some((key, _)) = definitions.definition_at(entry.item_definition_ref) else {
            continue;
        };
        let Some(client_id) = key
            .strip_prefix("oteryn:item.tibia.i")
            .and_then(|id| id.parse::<u32>().ok())
        else {
            continue;
        };
        if client_id == AXE {
            continue;
        }
        let slot =
            carried
                .entry(client_id)
                .or_insert((entry.item_definition_ref.get(), 0, Vec::new()));
        slot.1 += entry.count.get();
        slot.2.push((entry.handle, entry.count.get()));
    }
    let summary = carried
        .iter()
        .map(|(id, (_, units, _))| format!("{id}x{units}"))
        .collect::<Vec<_>>();
    let mut candidates = carried
        .iter()
        .filter(|(_, (_, units, _))| *units >= 2)
        .map(|(id, (definition, units, _))| (*id, *definition, *units))
        .collect::<Vec<_>>();
    candidates.sort_by_key(|&(id, _, units)| (std::cmp::Reverse(units), id));
    let mut marketable = Vec::new();
    let mut probes = Vec::new();
    for (client_id, definition, units) in candidates {
        let reply = market(&mut bots[A], request(2, definition, ZERO, 0, 0)).await?;
        probes.push(format!("{client_id}:code{}", reply.code));
        match reply.code {
            1 => marketable.push((client_id, definition, units)),
            14 => {}
            code => {
                return Err(format!(
                    "A book probe for {client_id} replied code={code} {}",
                    explain(code)
                )
                .into());
            }
        }
        if marketable.len() == 2 {
            break;
        }
    }
    let (x, y) = match marketable.as_slice() {
        [x, y, ..] => (*x, *y),
        [x] if x.2 >= 3 => (*x, *x),
        _ => {
            return Err(format!(
                "A carries no two marketable wares of >=2 units (or one of >=3);carried={summary:?};probes={probes:?}"
            )
            .into());
        }
    };
    let x = Ware {
        client_id: x.0,
        definition: x.1,
    };
    let y = Ware {
        client_id: y.0,
        definition: y.1,
    };
    // Recorded before the first deposit so cleanup retrieves whatever reached the depot.
    state.wares = Some((x, y));
    let mut needs = BTreeMap::new();
    *needs.entry(x.client_id).or_insert(0_u32) += 2;
    *needs.entry(y.client_id).or_insert(0_u32) += 1;
    let mut moves = Vec::new();
    for (client_id, need) in needs {
        let (before, _) = holdings(&mut bots[A], client_id).await?;
        let handles = carried
            .get(&client_id)
            .map(|slot| slot.2.clone())
            .unwrap_or_default();
        let mut moved = 0_u32;
        for (handle, count) in handles {
            if moved >= need {
                break;
            }
            let reply = depot_intent(
                &mut bots[A],
                depot::DepotIntent {
                    town: Some(town),
                    box_number: 1,
                    page: 0,
                    action: depot::DepotAction::Deposit(handle),
                },
            )
            .await?;
            if reply.outcome != depot::DepotOutcome::Moved {
                return Err(format!(
                    "A deposit of {client_id} replied {:?};source:crates/gameplay/dispatch/src/gameplay_transport/depot.rs",
                    reply.outcome
                )
                .into());
            }
            let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
            bots::wait_observation(
                bots,
                deadline,
                "MP04 deposit left backpack",
                move |bots, _| {
                    Ok(bots[A]
                        .session
                        .as_ref()
                        .is_some_and(|client| !holds_handle(client, handle)))
                },
            )
            .await?;
            moved += count;
        }
        let (after, _) = holdings(&mut bots[A], client_id).await?;
        if moved < need || after != before + moved {
            return Err(format!(
                "A depot {client_id}: moved {moved} of {need}, depot search {before}->{after}"
            )
            .into());
        }
        moves.push(format!("{client_id}:depot{before}->{after}"));
    }
    Ok(format!(
        "X={} Y={} {} carried={summary:?} probes={probes:?}",
        x.client_id,
        y.client_id,
        moves.join(",")
    ))
}

async fn sell_offer(bots: &mut [Bot<'_, '_>], ware: Ware, state: &mut State) -> TestResult<String> {
    let bank0 = bank(&mut bots[A]).await?;
    let (depot0, _) = holdings(&mut bots[A], ware.client_id).await?;
    let reply = create(
        bots,
        A,
        request(6, ware.definition, ZERO, 2, SELL_PRICE),
        state,
    )
    .await?;
    if reply.code != 1 || reply.action != 6 {
        return Err(format!(
            "A sell {}x2@{SELL_PRICE} replied action={} code={} {};A_bank={bank0};A_depot={depot0}",
            ware.client_id,
            reply.action,
            reply.code,
            explain(reply.code)
        )
        .into());
    }
    let id = reply.offer;
    state.sell = Some(id);
    let bank1 = bank(&mut bots[A]).await?;
    let (depot1, _) = holdings(&mut bots[A], ware.client_id).await?;
    let offers = own_offers(&mut bots[A]).await?;
    let charged = fee(2 * SELL_PRICE);
    let row = offers.iter().find(|row| row.offer == id);
    if bank0.checked_sub(bank1) != Some(charged)
        || depot0.checked_sub(depot1) != Some(2)
        || !row.is_some_and(|row| {
            row.definition == ware.definition
                && row.side == 2
                && row.amount == 2
                && row.price == SELL_PRICE
                && row.state == 1
        })
    {
        return Err(format!(
            "A sell offer {} state: bank {bank0}->{bank1} (fee {charged}), depot {depot0}->{depot1}, own row {row:?}",
            hex(&id)
        )
        .into());
    }
    Ok(format!(
        "offer={} item={} amount=2 price={SELL_PRICE} fee={charged} A_bank={bank0}->{bank1} A_depot={depot0}->{depot1}",
        hex(&id),
        ware.client_id
    ))
}

async fn browse(bots: &mut [Bot<'_, '_>], ware: Ware, id: [u8; 16]) -> TestResult<String> {
    let rows = query(&mut bots[B], 2, ware.definition, 62).await?;
    let row = rows.iter().find(|row| row.offer == id);
    if !row.is_some_and(|row| {
        row.definition == ware.definition
            && row.side == 2
            && row.amount == 2
            && row.price == SELL_PRICE
            && row.state == 1
    }) {
        return Err(format!(
            "B book for {} lacks A's offer {}: row {row:?} of {}",
            ware.client_id,
            hex(&id),
            rows.len()
        )
        .into());
    }
    Ok(format!(
        "B book item={} offer={} side=sell amount=2 price={SELL_PRICE} rows={}",
        ware.client_id,
        hex(&id),
        rows.len()
    ))
}

async fn partial_purchase(
    bots: &mut [Bot<'_, '_>],
    ware: Ware,
    id: [u8; 16],
    state: &mut State,
) -> TestResult<(String, Before)> {
    let before = Before {
        banks: [bank(&mut bots[A]).await?, bank(&mut bots[B]).await?],
        inbox: holdings(&mut bots[B], ware.client_id).await?.1,
    };
    let reply = market(&mut bots[B], request(8, 0, id, 1, 0)).await?;
    if reply.code != 1 || reply.action != 8 {
        return Err(format!(
            "B accept 1 of {} replied action={} code={} {};B_bank={}",
            hex(&id),
            reply.action,
            reply.code,
            explain(reply.code),
            before.banks[B]
        )
        .into());
    }
    state.bought = true;
    Ok((format!("B accepted 1 of offer={} code=1", hex(&id)), before))
}

async fn settle(
    bots: &mut [Bot<'_, '_>],
    ware: Ware,
    id: [u8; 16],
    before: Before,
) -> TestResult<String> {
    let a = bank(&mut bots[A]).await?;
    let b = bank(&mut bots[B]).await?;
    let (_, inbox) = holdings(&mut bots[B], ware.client_id).await?;
    let remaining = own_offers(&mut bots[A])
        .await?
        .iter()
        .find(|row| row.offer == id)
        .map(|row| row.amount);
    if a != before.banks[A] + SELL_PRICE
        || before.banks[B].checked_sub(b) != Some(SELL_PRICE)
        || inbox != before.inbox + 1
        || remaining != Some(1)
    {
        return Err(format!(
            "settlement A_bank {}->{a}, B_bank {}->{b}, B_inbox {}->{inbox}, A_remaining {remaining:?}",
            before.banks[A], before.banks[B], before.inbox
        )
        .into());
    }
    Ok(format!(
        "A_bank={}->{a} B_bank={}->{b} B_inbox_{}={}->{inbox} A_remaining=1",
        before.banks[A], before.banks[B], ware.client_id, before.inbox
    ))
}

/// A cancelled sell offer's items go to the seller's Inbox, not a depot box
/// (apps/game-server/migrations/0191_market_settlement.sql:418).
async fn cancel_remainder(
    bots: &mut [Bot<'_, '_>],
    ware: Ware,
    id: [u8; 16],
    state: &mut State,
) -> TestResult<String> {
    let remaining = own_offers(&mut bots[A])
        .await?
        .iter()
        .find(|row| row.offer == id)
        .map(|row| row.amount)
        .ok_or_else(|| format!("A offer {} not open before cancel", hex(&id)))?;
    let bank0 = bank(&mut bots[A]).await?;
    let (_, inbox0) = holdings(&mut bots[A], ware.client_id).await?;
    let reply = cancel(bots, A, id, state).await?;
    if reply.code != 1 || reply.action != 7 {
        return Err(format!(
            "A cancel {} replied action={} code={} {}",
            hex(&id),
            reply.action,
            reply.code,
            explain(reply.code)
        )
        .into());
    }
    state.cancelled = true;
    let bank1 = bank(&mut bots[A]).await?;
    let (_, inbox1) = holdings(&mut bots[A], ware.client_id).await?;
    let still = own_offers(&mut bots[A])
        .await?
        .iter()
        .any(|row| row.offer == id);
    if inbox1 != inbox0 + remaining || bank1 != bank0 || still {
        return Err(format!(
            "A cancel {}: Inbox {inbox0}->{inbox1} (expected +{remaining}), bank {bank0}->{bank1}, still listed {still}",
            hex(&id)
        )
        .into());
    }
    Ok(format!(
        "offer={} remaining={remaining} returned to A_inbox_{}={inbox0}->{inbox1} A_bank={bank1} unlisted",
        hex(&id),
        ware.client_id
    ))
}

async fn buy_offer(bots: &mut [Bot<'_, '_>], ware: Ware, state: &mut State) -> TestResult<String> {
    let bank0 = bank(&mut bots[B]).await?;
    let reply = create(
        bots,
        B,
        request(5, ware.definition, ZERO, 1, BUY_PRICE),
        state,
    )
    .await?;
    if reply.code != 1 || reply.action != 5 {
        return Err(format!(
            "B buy {}x1@{BUY_PRICE} replied action={} code={} {};B_bank={bank0}",
            ware.client_id,
            reply.action,
            reply.code,
            explain(reply.code)
        )
        .into());
    }
    let id = reply.offer;
    state.buy = Some(id);
    let bank1 = bank(&mut bots[B]).await?;
    let offers = own_offers(&mut bots[B]).await?;
    let row = offers.iter().find(|row| row.offer == id);
    let charged = fee(BUY_PRICE);
    if bank0.checked_sub(bank1) != Some(BUY_PRICE + charged)
        || !row.is_some_and(|row| {
            row.definition == ware.definition
                && row.side == 1
                && row.amount == 1
                && row.price == BUY_PRICE
                && row.state == 1
        })
    {
        return Err(format!(
            "B buy offer {}: bank {bank0}->{bank1} (escrow {BUY_PRICE} + fee {charged}), own row {row:?}",
            hex(&id)
        )
        .into());
    }
    Ok(format!(
        "offer={} item={} amount=1 price={BUY_PRICE} fee={charged} B_bank={bank0}->{bank1}",
        hex(&id),
        ware.client_id
    ))
}

async fn sell_into_buy(
    bots: &mut [Bot<'_, '_>],
    ware: Ware,
    id: [u8; 16],
    state: &mut State,
) -> TestResult<String> {
    let book = query(&mut bots[A], 2, ware.definition, 62).await?;
    if !book
        .iter()
        .any(|row| row.offer == id && row.side == 1 && row.amount == 1 && row.price == BUY_PRICE)
    {
        return Err(format!(
            "A book for {} lacks B's buy offer {} among {} rows",
            ware.client_id,
            hex(&id),
            book.len()
        )
        .into());
    }
    let a0 = bank(&mut bots[A]).await?;
    let b0 = bank(&mut bots[B]).await?;
    let (depot0, _) = holdings(&mut bots[A], ware.client_id).await?;
    let (_, inbox0) = holdings(&mut bots[B], ware.client_id).await?;
    let reply = market(&mut bots[A], request(8, 0, id, 1, 0)).await?;
    if reply.code != 1 || reply.action != 8 {
        return Err(format!(
            "A accept B's buy {} replied action={} code={} {};A_depot={depot0}",
            hex(&id),
            reply.action,
            reply.code,
            explain(reply.code)
        )
        .into());
    }
    state.sold = true;
    state.open.retain(|&(owner, open)| owner != B || open != id);
    let a1 = bank(&mut bots[A]).await?;
    let b1 = bank(&mut bots[B]).await?;
    let (depot1, _) = holdings(&mut bots[A], ware.client_id).await?;
    let (_, inbox1) = holdings(&mut bots[B], ware.client_id).await?;
    let listed = own_offers(&mut bots[B])
        .await?
        .iter()
        .any(|row| row.offer == id);
    if a1 != a0 + BUY_PRICE
        || b1 != b0
        || depot0.checked_sub(depot1) != Some(1)
        || inbox1 != inbox0 + 1
        || listed
    {
        return Err(format!(
            "fill {}: A_bank {a0}->{a1}, B_bank {b0}->{b1}, A_depot {depot0}->{depot1}, B_inbox {inbox0}->{inbox1}, still listed {listed}",
            hex(&id)
        )
        .into());
    }
    Ok(format!(
        "offer={} A_bank={a0}->{a1} A_depot_{}={depot0}->{depot1} B_inbox={inbox0}->{inbox1} B_bank={b1} unlisted",
        hex(&id),
        ware.client_id
    ))
}

fn trades(rows: &[MarketRow], id: [u8; 16]) -> Vec<(u32, u64)> {
    let mut found = rows
        .iter()
        .filter(|row| row.offer == id)
        .map(|row| (row.amount, row.price))
        .collect::<Vec<_>>();
    found.sort_unstable();
    found
}

/// An accept writes a buyer and a seller row keyed by the maker offer; a cancel writes the
/// maker's remaining units (apps/game-server/migrations/0191_market_settlement.sql:428,431).
async fn histories(bots: &mut [Bot<'_, '_>], sell: [u8; 16], buy: [u8; 16]) -> TestResult<String> {
    let a = history(&mut bots[A]).await?;
    let b = history(&mut bots[B]).await?;
    let observed = [
        trades(&a, sell),
        trades(&a, buy),
        trades(&b, sell),
        trades(&b, buy),
    ];
    let expected: [Vec<(u32, u64)>; 4] = [
        vec![(1, SELL_PRICE), (1, SELL_PRICE)],
        vec![(1, BUY_PRICE)],
        vec![(1, SELL_PRICE)],
        vec![(1, BUY_PRICE)],
    ];
    if observed != expected {
        return Err(format!(
            "history [A_sell, A_buy, B_sell, B_buy] observed {observed:?} expected {expected:?}"
        )
        .into());
    }
    Ok(format!(
        "A: sale+cancel of {} and fill of {}; B: purchase of {} and fill of {} (amount,price)={observed:?}",
        hex(&sell),
        hex(&buy),
        hex(&sell),
        hex(&buy)
    ))
}

async fn excess_items(
    bots: &mut [Bot<'_, '_>],
    ware: Ware,
    state: &mut State,
) -> TestResult<String> {
    let bank0 = bank(&mut bots[A]).await?;
    let held0 = holdings(&mut bots[A], ware.client_id).await?;
    let offers0 = own_offers(&mut bots[A]).await?;
    let amount = held0.0 + 1;
    let reply = create(bots, A, request(6, ware.definition, ZERO, amount, 1), state).await?;
    if reply.code != 3 {
        return Err(format!(
            "A sell {amount} of {} with depot {} expected refusal code 3, replied action={} code={} {}",
            ware.client_id,
            held0.0,
            reply.action,
            reply.code,
            explain(reply.code)
        )
        .into());
    }
    let bank1 = bank(&mut bots[A]).await?;
    let held1 = holdings(&mut bots[A], ware.client_id).await?;
    let offers1 = own_offers(&mut bots[A]).await?;
    if bank1 != bank0 || held1 != held0 || offers1 != offers0 {
        return Err(format!(
            "refusal changed state: bank {bank0}->{bank1}, (depot,inbox) {held0:?}->{held1:?}, offers {}->{}",
            offers0.len(),
            offers1.len()
        )
        .into());
    }
    Ok(format!(
        "code=3 amount={amount} depot={} A_bank={bank0} offers={} unchanged",
        held0.0,
        offers0.len()
    ))
}

async fn insufficient_gold(
    bots: &mut [Bot<'_, '_>],
    ware: Ware,
    state: &mut State,
) -> TestResult<String> {
    let bank0 = bank(&mut bots[B]).await?;
    let offers0 = own_offers(&mut bots[B]).await?;
    let price = bank0 + 1;
    if price > 999_999_999_999 {
        return Err(format!("B bank {bank0} leaves no encodable unaffordable price").into());
    }
    let reply = create(bots, B, request(5, ware.definition, ZERO, 1, price), state).await?;
    if reply.code != 2 {
        return Err(format!(
            "B buy 1 of {} at {price} with bank {bank0} expected refusal code 2, replied action={} code={} {}",
            ware.client_id,
            reply.action,
            reply.code,
            explain(reply.code)
        )
        .into());
    }
    let bank1 = bank(&mut bots[B]).await?;
    let offers1 = own_offers(&mut bots[B]).await?;
    if bank1 != bank0 || offers1 != offers0 {
        return Err(format!(
            "refusal changed state: bank {bank0}->{bank1}, offers {}->{}",
            offers0.len(),
            offers1.len()
        )
        .into());
    }
    Ok(format!(
        "code=2 price={price} B_bank={bank0} offers={} unchanged",
        offers0.len()
    ))
}

async fn fill_to_cap(
    bots: &mut [Bot<'_, '_>],
    ware: Ware,
    room: usize,
    placed: &mut Vec<[u8; 16]>,
    state: &mut State,
) -> TestResult<MarketReply> {
    for _ in 0..room {
        let reply = create(bots, A, request(5, ware.definition, ZERO, 1, 1), state).await?;
        if reply.code != 1 {
            return Err(format!(
                "A filler offer {} replied code={} {}",
                placed.len() + 1,
                reply.code,
                explain(reply.code)
            )
            .into());
        }
        placed.push(reply.offer);
    }
    create(bots, A, request(5, ware.definition, ZERO, 1, 1), state).await
}

async fn offer_cap(bots: &mut [Bot<'_, '_>], ware: Ware, state: &mut State) -> TestResult<String> {
    let open = own_offers(&mut bots[A]).await?.len();
    let bank0 = bank(&mut bots[A]).await?;
    let room = OFFER_CAP.saturating_sub(open);
    let each = 1 + fee(1);
    let need = u64::try_from(room)? * each;
    if bank0 < need {
        // Still ask the server once so the row carries its real answer.
        let probe = create(bots, A, request(5, ware.definition, ZERO, 1, 1), state).await?;
        if probe.code == 1 {
            cancel(bots, A, probe.offer, state).await?;
        }
        return Err(format!(
            "prerequisite: A bank {bank0} < {need} gold for {room} more 1x1 buy offers ({each} each) below cap {OFFER_CAP}; probe placement replied code={} {}",
            probe.code,
            explain(probe.code)
        )
        .into());
    }
    let mut placed = Vec::new();
    let over = fill_to_cap(bots, ware, room, &mut placed, state).await;
    let mut failures = Vec::new();
    for id in placed.iter().copied() {
        match cancel(bots, A, id, state).await {
            Ok(reply) if reply.code == 1 => {}
            Ok(reply) => failures.push(format!("cancel {} code={}", hex(&id), reply.code)),
            Err(error) => failures.push(error.to_string()),
        }
    }
    let over = over?;
    if over.code == 1 {
        cancel(bots, A, over.offer, state).await?;
    }
    let after = own_offers(&mut bots[A]).await?.len();
    if over.code != 5 || !failures.is_empty() || after != open {
        return Err(format!(
            "offer {} beyond cap {OFFER_CAP} replied code={} {}; cleanup {failures:?}; open {open}->{after}",
            open + room + 1,
            over.code,
            explain(over.code)
        )
        .into());
    }
    Ok(format!(
        "code=5 at offer {} after {room} placements from {open} open; all {room} cancelled",
        OFFER_CAP + 1
    ))
}

type Snapshot = (u64, (u32, u32), (u32, u32), Vec<MarketRow>, Vec<MarketRow>);

async fn snapshot(bot: &mut Bot<'_, '_>, x: Ware, y: Ware) -> TestResult<Snapshot> {
    Ok((
        bank(bot).await?,
        holdings(bot, x.client_id).await?,
        holdings(bot, y.client_id).await?,
        history(bot).await?,
        own_offers(bot).await?,
    ))
}

async fn relog(
    bots: &mut [Bot<'_, '_>],
    world: &BundleWorld,
    x: Ware,
    y: Ware,
    state: &State,
) -> TestResult<String> {
    let before = [
        snapshot(&mut bots[A], x, y).await?,
        snapshot(&mut bots[B], x, y).await?,
    ];
    let sessions = [
        bots[A].client()?.join_snapshot().game_session_id,
        bots[B].client()?.join_snapshot().game_session_id,
    ];
    bots::relog_all(bots).await?;
    for index in [A, B] {
        if bots[index].client()?.join_snapshot().game_session_id == sessions[index] {
            return Err(format!("{} relog kept its game session", bots[index].name).into());
        }
        let at = tile(bots[index].client()?.world_spatial().actor_position)?;
        if !near_locker(world, at) {
            reach_locker(bots, index, world).await?;
        }
    }
    let after = [
        snapshot(&mut bots[A], x, y).await?,
        snapshot(&mut bots[B], x, y).await?,
    ];
    for index in [A, B] {
        if before[index] != after[index] {
            return Err(format!(
                "{} market state changed across relog: before {:?} after {:?}",
                bots[index].name, before[index], after[index]
            )
            .into());
        }
    }
    let summary = format!(
        "A(bank={},X={:?},Y={:?},history={},offers={}) B(bank={},X={:?},Y={:?},history={},offers={})",
        after[A].0,
        after[A].1,
        after[A].2,
        after[A].3.len(),
        after[A].4.len(),
        after[B].0,
        after[B].1,
        after[B].2,
        after[B].3.len(),
        after[B].4.len()
    );
    if !(state.bought && state.cancelled && state.sold) {
        return Err(format!(
            "both relogged with equal state {summary}, but no completed trade exists to persist (blocked by the trade rows)"
        )
        .into());
    }
    Ok(format!("new sessions; unchanged {summary}"))
}

async fn cleanup(bots: &mut [Bot<'_, '_>], state: &mut State) -> TestResult<String> {
    let mut failures = Vec::new();
    let mut cancelled = 0;
    for (index, id) in state.open.clone() {
        match cancel(bots, index, id, state).await {
            Ok(reply) if matches!(reply.code, 1 | 6) => cancelled += 1,
            Ok(reply) => failures.push(format!(
                "{} cancel {} code={} {}",
                bots[index].name,
                hex(&id),
                reply.code,
                explain(reply.code)
            )),
            Err(error) => failures.push(error.to_string()),
        }
    }
    let mut retrieved = Vec::new();
    if let Some((x, y)) = state.wares {
        let mut ids = vec![x.client_id, y.client_id];
        ids.dedup();
        for client_id in ids {
            match search_command(
                &mut bots[A],
                search::Request::Retrieve {
                    client_id,
                    all: true,
                },
            )
            .await
            {
                Ok(reply)
                    if matches!(
                        reply.outcome,
                        search::Outcome::Retrieved | search::Outcome::NotFound
                    ) =>
                {
                    retrieved.push(format!("{client_id}:{}", reply.retrieved));
                }
                Ok(reply) => {
                    failures.push(format!(
                        "A retrieve {client_id} outcome {:?}",
                        reply.outcome
                    ));
                }
                Err(error) => failures.push(error.to_string()),
            }
        }
    }
    if !failures.is_empty() {
        return Err(failures.join("; ").into());
    }
    Ok(format!(
        "cancelled={cancelled} A_retrieved_to_backpack={retrieved:?}"
    ))
}

async fn journey(bots: &mut [Bot<'_, '_>], world: &BundleWorld, failures: &mut Vec<String>) {
    let mut state = State::default();
    if !check(
        "locker",
        bounded("locker", 240, lockers(bots, world, &mut state)).await,
        failures,
    ) {
        for step in &ROWS[1..] {
            blocked(step, "locker", failures);
        }
        return;
    }
    check("open", bounded("open", 30, open(bots)).await, failures);
    let stocked = check(
        "deposit",
        bounded("deposit", 120, stock(bots, &mut state)).await,
        failures,
    );
    match state.wares.filter(|_| stocked) {
        Some((x, y)) => {
            check(
                "sell_offer",
                bounded("sell_offer", 60, sell_offer(bots, x, &mut state)).await,
                failures,
            );
            if let Some(id) = state.sell {
                check(
                    "browse",
                    bounded("browse", 60, browse(bots, x, id)).await,
                    failures,
                );
                match bounded(
                    "partial_purchase",
                    60,
                    partial_purchase(bots, x, id, &mut state),
                )
                .await
                {
                    Ok((detail, before)) => {
                        check("partial_purchase", Ok(detail), failures);
                        check(
                            "settlement",
                            bounded("settlement", 60, settle(bots, x, id, before)).await,
                            failures,
                        );
                    }
                    Err(error) => {
                        check("partial_purchase", Err(error), failures);
                        blocked("settlement", "partial_purchase", failures);
                    }
                }
                check(
                    "cancel_remainder",
                    bounded(
                        "cancel_remainder",
                        60,
                        cancel_remainder(bots, x, id, &mut state),
                    )
                    .await,
                    failures,
                );
            } else {
                for step in [
                    "browse",
                    "partial_purchase",
                    "settlement",
                    "cancel_remainder",
                ] {
                    blocked(step, "sell_offer", failures);
                }
            }
            check(
                "buy_offer",
                bounded("buy_offer", 60, buy_offer(bots, y, &mut state)).await,
                failures,
            );
            if let Some(id) = state.buy {
                check(
                    "sell_into_buy",
                    bounded("sell_into_buy", 60, sell_into_buy(bots, y, id, &mut state)).await,
                    failures,
                );
            } else {
                blocked("sell_into_buy", "buy_offer", failures);
            }
            match (state.sell, state.buy) {
                (Some(sell), Some(buy)) if state.bought && state.cancelled && state.sold => {
                    check(
                        "history_both_sides",
                        bounded("history_both_sides", 60, histories(bots, sell, buy)).await,
                        failures,
                    );
                }
                _ => blocked("history_both_sides", "the trade rows", failures),
            }
            check(
                "excess_items_refused",
                bounded(
                    "excess_items_refused",
                    60,
                    excess_items(bots, x, &mut state),
                )
                .await,
                failures,
            );
            check(
                "insufficient_gold_refused",
                bounded(
                    "insufficient_gold_refused",
                    60,
                    insufficient_gold(bots, x, &mut state),
                )
                .await,
                failures,
            );
            check(
                "offer_cap_refused",
                bounded("offer_cap_refused", 300, offer_cap(bots, x, &mut state)).await,
                failures,
            );
            check(
                "relog_persistence",
                bounded("relog_persistence", 600, relog(bots, world, x, y, &state)).await,
                failures,
            );
        }
        None => {
            for step in &ROWS[3..15] {
                blocked(step, "deposit", failures);
            }
        }
    }
    check(
        "cleanup",
        bounded("cleanup", 300, cleanup(bots, &mut state)).await,
        failures,
    );
}

pub(super) async fn run(
    core: &SeamClients<'_>,
    packets: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    let started = std::time::Instant::now();
    let mut failures = Vec::new();
    let (mut bots, setup) = tokio::time::timeout(
        Duration::from_secs(180),
        bots::connect_bots(&[(core, 0, None), (packets, 0, None)]),
    )
    .await
    .unwrap_or_else(|_| (Vec::new(), Err("MP04 admission deadline".into())));
    let ready = check(
        "admissions",
        setup.map(|()| "A=core/0 B=packets/0 admitted over the client wire".to_owned()),
        &mut failures,
    );
    if ready {
        journey(&mut bots, world, &mut failures).await;
    } else {
        for step in ROWS {
            blocked(step, "admissions", &mut failures);
        }
    }
    check(
        "release",
        bounded("release", 1800, bots::release_all(&mut bots, core, packets))
            .await
            .map(|()| "both Characters released".to_owned()),
        &mut failures,
    );
    let status = if failures.is_empty() { "PASS" } else { "FAIL" };
    evidence(&format!(
        "e2e_all scenario=MP04.full status={status} seconds={:.3}",
        started.elapsed().as_secs_f64()
    ));
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; ").into())
    }
}
