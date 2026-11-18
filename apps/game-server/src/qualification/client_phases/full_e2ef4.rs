//! E2EF4: real-world economy/social journeys, with fail-closed full-row evidence.
//! Every step is a native client command a player could send. No fixture provisioning, SQL
//! mutations, teleports, clock overrides or invented wire handles; a step the server refuses
//! fails the row with the server's own refusal as the GAP text.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityRef};
use oteryn_protocol_oteryn::npc_service as wire;

use verdict::Row;

#[path = "full_e2ef4_f4e05.rs"]
mod full_e2ef4_f4e05;

#[path = "full_e2ef4_f4e07.rs"]
mod full_e2ef4_f4e07;

// Detail stays on the GAP lines, separate from the report's single-token evidence field.
// A failure is never turned into MISSING-SERVER merely because a journey is unfinished.
const PASS: &str = "observed-all-required-actions";
const FAIL: &str = "required-actions-unproven-see-E2EF4_GAP";

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
];

pub(super) async fn run(
    client: &mut Option<DevClientSession>,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) {
    // Each row reports even when another row has missing content or a refused command.
    // Socket waits are bounded; a timed-out session is not evidence of server absence.
    macro_rules! row {
        ($secs:literal, $id:literal, $actions:ident, $journey:ident $(, $arg:expr)*) => {{
            let mut row = Row::new("E2EF4", $id, $actions).evidence(PASS, FAIL);
            let watch = row.watch();
            let result: TestResult = async {
                // An interrupted or poisoned exchange cannot be resumed by another row.
                let active = match &mut *client {
                    Some(active) => active,
                    slot => {
                        let _admission = watch.enter();
                        slot.insert(connect_character(clients, 0x89, 1).await?)
                    }
                };
                let mut completed = false;
                let result = watch.bound(
                    Duration::from_secs($secs),
                    "native-journey-deadline;remaining-assertions-unproven",
                    async {
                        let result = $journey(&mut row, active $(, $arg)*).await;
                        completed = true;
                        result
                    },
                ).await;
                if !completed {
                    *client = None;
                    return result;
                }
                // This checks the actual session even when cleanup wrapped the original error
                // or a successful logout was followed by a failed re-admission.
                if let Err(error) = active.service_liveness(Duration::from_millis(1)).await {
                    *client = None;
                    result?;
                    return Err(error.into());
                }
                result
            }.await;
            row.finish(&result);
        }};
    }
    row!(600, "E01", E01, shop, clients, world, npcs);
    row!(180, "E03", E03, coins, clients, world, npcs);
    row!(240, "E04", E04, trade, clients, world);
    row!(1200, "E05", E05, market, clients, world);
    row!(240, "E07", E07, storage, clients, world);
    row!(90, "S01", S01, party, clients, world);
    row!(1500, "S03", S03, guild, clients, world, npcs);
    row!(1500, "S09", S09, logout, clients, world);
    row!(240, "S18", S18, speech, clients, world);
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

// Adapt native command errors to the existing observer-liveness driver.
async fn while_serving<T, E>(
    idle: &mut DevClientSession,
    operation: impl std::future::Future<Output = Result<T, E>>,
) -> TestResult<T>
where
    E: Into<Box<dyn std::error::Error>>,
{
    super::full_e2ef5::with_observer_liveness(idle, async { operation.await.map_err(Into::into) })
        .await
}

fn own(client: &DevClientSession) -> TestResult<EntityRef> {
    let entities = client.world_entities().ok_or("native entities absent")?;
    entities
        .iter()
        .find(|entity| &entity.entity.identity == entities.own_identity())
        .map(|entity| entity.entity)
        .ok_or_else(|| "native own actor absent".into())
}

async fn approach_peer_tile(
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
        progress();
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

/// Keeps the no-progress watchdog fed during a long journey, once its first action has passed.
fn beat(_row: &mut Row) {
    verdict::Harness::global().progress("e2ef4-journey");
}

/// Progress for loops that have no `Row` in reach.
fn progress() {
    verdict::Harness::global().progress("e2ef4-wait");
}

fn coin_refs() -> TestResult<[std::num::NonZeroU32; 3]> {
    let index = provisioning::item_index()?;
    Ok([
        provisioning::item_reference(&index, 3031)?,
        provisioning::item_reference(&index, 3035)?,
        provisioning::item_reference(&index, 3043)?,
    ])
}

fn coin_counts(units: &BTreeMap<u32, u64>, refs: &[std::num::NonZeroU32; 3]) -> [u64; 3] {
    refs.map(|reference| units.get(&reference.get()).copied().unwrap_or(0))
}

/// Gold value of the carried coins (platinum is 100 gold, crystal 10000 gold).
fn wealth(units: &BTreeMap<u32, u64>, refs: &[std::num::NonZeroU32; 3]) -> u64 {
    let [gold, platinum, crystal] = coin_counts(units, refs);
    gold + 100 * platinum + 10_000 * crystal
}

fn non_coin(units: &BTreeMap<u32, u64>, refs: &[std::num::NonZeroU32; 3]) -> BTreeMap<u32, u64> {
    units
        .iter()
        .filter(|(definition, _)| !refs.iter().any(|r| r.get() == **definition))
        .map(|(definition, count)| (*definition, *count))
        .collect()
}

/// Direct backpack entries only: what a player can offer to an NPC or another player.
fn carried_entries(client: &DevClientSession) -> TestResult<BTreeMap<u32, u64>> {
    let inventory = client.inventory().ok_or("native inventory absent")?;
    let mut units = BTreeMap::new();
    for item in &inventory.entries {
        *units.entry(item.item_definition_ref.get()).or_default() += u64::from(item.count.get());
    }
    Ok(units)
}

async fn settle(
    client: &mut DevClientSession,
    step: &str,
    accept: impl Fn(&BTreeMap<u32, u64>) -> bool,
) -> TestResult<BTreeMap<u32, u64>> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    loop {
        let now = inventory_units(client)?;
        if accept(&now) {
            return Ok(now);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "{step}: carried custody did not reach the expected quantities within 8s; observed={now:?}"
            )
            .into());
        }
        progress();
        client.service_liveness(Duration::from_millis(20)).await?;
        client.take_events();
    }
}

struct Spoken {
    conversation: wire::NpcConversation,
    window: Option<wire::NpcTradeWindow>,
}

/// One native NPC talk command. The conversation (and for `trade` the window) must arrive as
/// deltas sequenced after the command's own reply.
async fn speak(client: &mut DevClientSession, npc: EntityRef, text: &str) -> TestResult<Spoken> {
    use oteryn_dev_client::SessionEvent;
    // The NPC talk rate window refuses commands sent back to back.
    tokio::time::sleep(Duration::from_millis(260)).await;
    client.take_events();
    let payload = wire::encode_npc_talk_intent(&wire::NpcTalkIntent {
        npc_actor: npc,
        text: text.into(),
    })
    .map_err(|e| format!("NPC {text:?}: encoder refused: {e:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_NPC_SERVICE_V1,
            wire::COMMAND_TYPE_NPC_TALK_INTENT,
            &payload,
        )
        .await?;
    let disposition = wire::decode_npc_intent_result(&reply.payload)
        .map_err(|e| format!("NPC {text:?}: reply decode: {e:?}"))?;
    if reply.status != oteryn_protocol_oteryn::CommandStatus::Accepted
        || disposition != wire::NpcIntentDisposition::Ok
    {
        return Err(format!(
            "NPC refused {text:?}: status={:?} disposition={disposition:?}",
            reply.status
        )
        .into());
    }
    let wants_window = text == "trade";
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    let (mut conversation, mut window) = (None, None);
    loop {
        for event in client.take_events() {
            match event {
                SessionEvent::NpcConversation(delta)
                    if delta.server_sequence > reply.server_sequence =>
                {
                    conversation = Some(delta.value);
                }
                SessionEvent::NpcTradeWindow(delta)
                    if delta.server_sequence > reply.server_sequence =>
                {
                    window = Some(delta.value);
                }
                _ => {}
            }
        }
        if conversation.is_some()
            && (!wants_window || window.is_some())
            && let Some(conversation) = conversation
        {
            return Ok(Spoken {
                conversation,
                window,
            });
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "NPC {text:?} accepted at sequence {} but its conversation/window delta never arrived",
                reply.server_sequence
            )
            .into());
        }
        progress();
        client.service_liveness(Duration::from_millis(25)).await?;
    }
}

fn npc_actor(
    client: &mut DevClientSession,
    npc: &oteryn_world_bundle::npc::Npc,
) -> TestResult<EntityRef> {
    let basis = spatial_basis::current(client)?;
    client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entity| {
                entity.kind == oteryn_dev_client::EntityKind::Npc
                    && npc.placements.iter().any(|p| {
                        entity.position.x == i32::from(p.x)
                            && entity.position.y == i32::from(p.y)
                            && entity.position.floor == basis.floor(p.floor)
                    })
            })
        })
        .map(|entity| entity.entity)
        .ok_or_else(|| format!("{} actor not visible at an authored placement", npc.key).into())
}

async fn approach_npc(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npc: &oteryn_world_bundle::npc::Npc,
) -> TestResult<EntityRef> {
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        npc.placements
            .iter()
            .any(|p| p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3)
    })
    .ok_or_else(|| format!("{} unreachable by an authored route", npc.key))?;
    walk(client, world, &path).await?;
    npc_actor(client, npc)
}

/// The display name a character sees for itself; the transfer recipient is read from the
/// recipient's own view, never from a watcher's possibly stale entity list.
fn own_name(client: &DevClientSession) -> TestResult<String> {
    let entities = client.world_entities().ok_or("own entity domain missing")?;
    let name = entities
        .iter()
        .find(|entity| entity.entity.identity == *entities.own_identity())
        .and_then(|entity| entity.display_name.as_ref())
        .ok_or("own display name missing")?;
    Ok(name
        .as_str()
        .map_err(|error| format!("invalid display name: {error:?}"))?
        .to_owned())
}

fn balance_of(lines: &[String]) -> Option<u64> {
    lines.iter().rev().find_map(|line| {
        line.strip_prefix("Your account balance is ")?
            .strip_suffix(" gold.")?
            .parse()
            .ok()
    })
}

/// Walks to each reachable NPC in turn and asks for the bank balance. An NPC that answers with
/// a balance is the banker; a bank refusal is returned as the failure, verbatim.
async fn find_banker<'a>(
    row: &mut Row,
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &'a oteryn_world_bundle::npc::Table,
) -> TestResult<(&'a oteryn_world_bundle::npc::Npc, EntityRef, u64)> {
    let mut attempts = Vec::new();
    for npc in &npcs.npcs {
        beat(row);
        if route(world, tile(client.world_spatial().actor_position)?, |at| {
            npc.placements
                .iter()
                .any(|p| p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3)
        })
        .is_none()
        {
            continue;
        }
        let Ok(actor) = approach_npc(client, world, npc).await else {
            continue;
        };
        if speak(client, actor, "hi").await.is_err() {
            continue;
        }
        let Ok(balance) = speak(client, actor, "balance").await else {
            continue;
        };
        if let Some(amount) = balance_of(&balance.conversation.lines) {
            return Ok((npc, actor, amount));
        }
        if let Some(line) = balance
            .conversation
            .lines
            .iter()
            .find(|line| line.contains("left the island"))
        {
            return Err(format!("bank refused by {}: {line}", npc.key).into());
        }
        attempts.push(format!("{}:{:?}", npc.key, balance.conversation.lines));
    }
    Err(format!("no reachable NPC answered a bank balance request: {attempts:?}").into())
}

async fn bank_exchange(
    client: &mut DevClientSession,
    actor: EntityRef,
    command: &str,
    prompt: &str,
    done: &str,
) -> TestResult {
    let asked = speak(client, actor, command).await?;
    if !asked.conversation.lines.iter().any(|line| line == prompt) {
        return Err(format!(
            "bank {command:?}: expected prompt {prompt:?}, server said {:?}",
            asked.conversation.lines
        )
        .into());
    }
    let confirmed = speak(client, actor, "yes").await?;
    if !confirmed.conversation.lines.iter().any(|line| line == done) {
        return Err(format!(
            "bank {command:?}: expected {done:?}, server said {:?}",
            confirmed.conversation.lines
        )
        .into());
    }
    Ok(())
}

async fn bank_balance(client: &mut DevClientSession, actor: EntityRef) -> TestResult<u64> {
    let reply = speak(client, actor, "balance").await?;
    balance_of(&reply.conversation.lines).ok_or_else(|| {
        format!(
            "bank balance reply carried no balance: {:?}",
            reply.conversation.lines
        )
        .into()
    })
}

async fn shop(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult {
    let mut peer = None;
    let result = shop_journey(row, client, &mut peer, clients, world, npcs).await;
    let Some(mut peer) = peer else {
        return result;
    };
    let (cleanup_result, cleanup, attempts, busy_timed_out) =
        match while_serving(client, super::logout_retrying_busy(&mut peer)).await {
            Ok(cleanup) => cleanup,
            Err(error) => return result.and(Err(error)),
        };
    if cleanup_result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("shop peer cleanup logout refused: {cleanup}; attempts={attempts}; busy_timed_out={busy_timed_out}; journey={result:?}").into());
    }
    result
}

fn sell_all_matches(present: u64, sold: u64, credited: u64, unit_price: u32) -> bool {
    present != 0 && sold == present && credited == present * u64::from(unit_price)
}

async fn shop_journey(
    row: &mut Row,
    client: &mut DevClientSession,
    peer: &mut Option<DevClientSession>,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult {
    use oteryn_dev_client::client_runtime::npc_shop as runtime;
    use oteryn_protocol_oteryn::npc_shop as shop;
    let refs = coin_refs()?;
    let sam = npcs
        .npcs
        .iter()
        .find(|npc| npc.key == "oteryn:npc.sam")
        .ok_or("content:oteryn:npc.sam absent")?;
    let actor = approach_npc(client, world, sam).await?;
    let hi = speak(client, actor, "hi").await?;
    if !hi
        .conversation
        .open
        .as_ref()
        .is_some_and(|(npc, _)| npc.identity == actor.identity)
    {
        return Err(format!(
            "greeting did not open conversation: open={:?} lines={:?}",
            hi.conversation.open, hi.conversation.lines
        )
        .into());
    }
    row.pass("greet", "greeting-delta-opens-conversation-with-sam");
    let trade = speak(client, actor, "trade").await?;
    let window = trade.window.ok_or("trade did not deliver a shop window")?;
    let revision = window.open.ok_or("trade window closed")?.1;
    let (index, offer) = window
        .offers
        .iter()
        .enumerate()
        .filter(|(_, o)| o.side == wire::NpcTradeSide::Buy)
        .min_by_key(|(_, o)| o.unit_price)
        .ok_or("Sam has no buy offer")?;
    let reply = runtime::send(
        client.session_mut(),
        &runtime::NpcShopCommand(shop::NpcShopIntent {
            npc_actor: actor,
            catalogue_revision: revision,
            offer_index: u32::try_from(index)?,
            side: offer.side,
            action: shop::NpcShopAction::Look,
            expected_unit_price: offer.unit_price,
        }),
    )
    .await?;
    if !reply
        .result
        .is_some_and(|r| r.disposition == wire::NpcIntentDisposition::Ok && !r.look_text.is_empty())
    {
        return Err("shop look did not return a description".into());
    }
    row.pass("look", "native-offer-description-nonempty");

    // SELL: one lot of a carried item Sam buys.
    let carried = carried_entries(client)?;
    let (index, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, o)| {
            o.side == wire::NpcTradeSide::Sell
                && carried.get(&o.item_definition_ref).copied().unwrap_or(0) >= u64::from(o.count)
        })
        .ok_or_else(|| {
            format!(
                "Sam buys none of the carried items: carried={carried:?} offers={:?}",
                window.offers
            )
        })?;
    let before = inventory_units(client)?;
    let disposition = runtime::buy(
        client.session_mut(),
        &wire::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision: revision,
            offer_index: u32::try_from(index)?,
            side: offer.side,
            quantity: 1,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if disposition != wire::NpcIntentDisposition::Ok {
        return Err(format!("sell refused by Sam: {disposition:?}").into());
    }
    let mut expected_items = non_coin(&before, &refs);
    let sold = expected_items
        .get_mut(&offer.item_definition_ref)
        .ok_or("sold definition not carried")?;
    *sold -= u64::from(offer.count);
    if *sold == 0 {
        expected_items.remove(&offer.item_definition_ref);
    }
    let expected_wealth = wealth(&before, &refs) + u64::from(offer.unit_price);
    settle(client, "sell", |now| {
        non_coin(now, &refs) == expected_items && wealth(now, &refs) == expected_wealth
    })
    .await?;
    row.pass("sell", "lot-removed-and-quoted-price-credited");
    beat(row);

    // SELL_ALL: every unit of another carried stack Sam buys.
    let carried = carried_entries(client)?;
    let (index, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, o)| {
            o.side == wire::NpcTradeSide::Sell
                && o.count == 1
                && carried.get(&o.item_definition_ref).copied().unwrap_or(0) >= 1
        })
        .ok_or_else(|| {
            format!(
                "no second sellable stack left for SELL_ALL: carried={carried:?} offers={:?}",
                window.offers
            )
        })?;
    let before = inventory_units(client)?;
    let reply = runtime::send(
        client.session_mut(),
        &runtime::NpcShopCommand(shop::NpcShopIntent {
            npc_actor: actor,
            catalogue_revision: revision,
            offer_index: u32::try_from(index)?,
            side: offer.side,
            action: shop::NpcShopAction::SellAll,
            expected_unit_price: offer.unit_price,
        }),
    )
    .await?;
    let outcome = reply.result.ok_or("SELL_ALL returned no result")?;
    if outcome.disposition != wire::NpcIntentDisposition::Ok {
        return Err(format!("SELL_ALL refused by Sam: {:?}", outcome.disposition).into());
    }
    let sold_units = u64::from(outcome.sold_units);
    let present = carried
        .get(&offer.item_definition_ref)
        .copied()
        .unwrap_or(0);
    if !sell_all_matches(
        present,
        sold_units,
        outcome.total_gold_units,
        offer.unit_price,
    ) {
        return Err(format!(
            "SELL_ALL result disagrees with the quote: sold={sold_units} present={present} gold={} unit_price={}",
            outcome.total_gold_units, offer.unit_price
        )
        .into());
    }
    let mut expected_items = non_coin(&before, &refs);
    let left = expected_items
        .get(&offer.item_definition_ref)
        .copied()
        .unwrap_or(0)
        .checked_sub(sold_units)
        .ok_or("SELL_ALL custody underflow")?;
    if left == 0 {
        expected_items.remove(&offer.item_definition_ref);
    } else {
        expected_items.insert(offer.item_definition_ref, left);
    }
    let expected_wealth = wealth(&before, &refs) + outcome.total_gold_units;
    settle(client, "sell-all", |now| {
        non_coin(now, &refs) == expected_items && wealth(now, &refs) == expected_wealth
    })
    .await?;
    row.pass("sell-all", "units-removed-and-total-credited");
    beat(row);

    // BUY: the cheapest buy offer the earned money covers.
    let before = inventory_units(client)?;
    let funds = wealth(&before, &refs);
    let (index, offer) = window
        .offers
        .iter()
        .enumerate()
        .filter(|(_, o)| o.side == wire::NpcTradeSide::Buy && u64::from(o.unit_price) <= funds)
        .min_by_key(|(_, o)| o.unit_price)
        .ok_or_else(|| {
            format!(
                "no buy offer is affordable with the {funds} gold earned by selling: offers={:?}",
                window.offers
            )
        })?;
    let disposition = runtime::buy(
        client.session_mut(),
        &wire::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision: revision,
            offer_index: u32::try_from(index)?,
            side: offer.side,
            quantity: 1,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if disposition != wire::NpcIntentDisposition::Ok {
        return Err(format!("buy refused by Sam: {disposition:?}").into());
    }
    let mut expected_items = non_coin(&before, &refs);
    *expected_items.entry(offer.item_definition_ref).or_default() += u64::from(offer.count);
    let expected_wealth = funds - u64::from(offer.unit_price);
    settle(client, "buy", |now| {
        non_coin(now, &refs) == expected_items && wealth(now, &refs) == expected_wealth
    })
    .await?;
    row.pass("buy", "lot-added-and-quoted-price-debited");
    beat(row);

    // BANK: deposit, withdraw, transfer to the real peer, then fresh admission.
    let (banker, bank, balance) = find_banker(row, client, world, npcs).await?;
    let funds = wealth(&inventory_units(client)?, &refs);
    if funds < 2 {
        return Err(format!("bank steps need at least 2 gold carried; earned {funds}").into());
    }
    let items = non_coin(&inventory_units(client)?, &refs);
    bank_exchange(
        client,
        bank,
        "deposit all",
        &format!("Do you want to deposit {funds} gold?"),
        &format!("You have deposited {funds} gold."),
    )
    .await?;
    settle(client, "deposit", |now| {
        wealth(now, &refs) == 0 && non_coin(now, &refs) == items
    })
    .await?;
    let deposited = bank_balance(client, bank).await?;
    if deposited != balance + funds {
        return Err(
            format!("deposit: balance {balance}+{funds} expected, bank says {deposited}").into(),
        );
    }
    row.pass("deposit", "backpack-coins-moved-to-exact-bank-balance");
    beat(row);

    let withdrawn = funds / 2;
    bank_exchange(
        client,
        bank,
        &format!("withdraw {withdrawn} gold"),
        &format!("Do you want to withdraw {withdrawn} gold?"),
        &format!("You have withdrawn {withdrawn} gold."),
    )
    .await?;
    settle(client, "withdraw", |now| {
        wealth(now, &refs) == withdrawn && non_coin(now, &refs) == items
    })
    .await?;
    let after_withdraw = bank_balance(client, bank).await?;
    if after_withdraw != deposited - withdrawn {
        return Err(format!(
            "withdraw: balance {deposited}-{withdrawn} expected, bank says {after_withdraw}"
        )
        .into());
    }
    row.pass("withdraw", "bank-balance-debited-and-backpack-credited");
    beat(row);

    // Admit the real recipient only when needed, and read its own balance before and after.
    let peer = peer.insert(while_serving(client, connect_character(clients, 0x87, 0)).await?);
    let recipient = own_name(peer)?;
    let (peer_actor, peer_before) = while_serving(client, async {
        let actor = approach_npc(peer, world, banker).await?;
        speak(peer, actor, "hi").await?;
        bank_balance(peer, actor)
            .await
            .map(|balance| (actor, balance))
    })
    .await?;
    let after_transfer = while_serving(peer, async {
        bank_exchange(
            client,
            bank,
            &format!("transfer 1 gold to {recipient}"),
            &format!("Do you want to transfer 1 gold to {recipient}?"),
            &format!("Very well. You have transferred 1 gold to {recipient}."),
        )
        .await?;
        bank_balance(client, bank).await
    })
    .await?;
    let peer_after = while_serving(client, bank_balance(peer, peer_actor)).await?;
    if after_transfer != after_withdraw - 1 || peer_after != peer_before + 1 {
        return Err(format!(
            "transfer: sender {after_withdraw}->{after_transfer}, recipient {peer_before}->{peer_after}"
        )
        .into());
    }
    row.pass("transfer", "sender-debited-and-real-recipient-credited");
    beat(row);

    while_serving(peer, async {
        let carried = inventory_units(client)?;
        relog(client, clients).await?;
        if inventory_units(client)? != carried {
            return Err(format!(
                "fresh admission changed carried custody: before={carried:?} after={:?}",
                inventory_units(client)?
            )
            .into());
        }
        let bank = approach_npc(client, world, banker).await?;
        speak(client, bank, "hi").await?;
        let remembered = bank_balance(client, bank).await?;
        if remembered != after_transfer {
            return Err(format!(
                "fresh admission changed the bank balance: {after_transfer}->{remembered}"
            )
            .into());
        }
        row.pass(
            "relog-inventory-bank",
            "fresh-admission-inventory-and-bank-balance-retained",
        );
        TestResult::Ok(())
    })
    .await
}

type CoinStep = (&'static str, usize, fn(u32) -> bool, usize, u64, usize, u64);

fn coin_steps() -> [CoinStep; 4] {
    // (action, coin used, input quantity filter, coin stack spent, coin stack gained, gained)
    [
        ("gold-up", 0, |q| q == 100, 0, 100, 1, 1),
        ("platinum-down", 1, |q| q != 100, 1, 1, 0, 100),
        ("crystal-down", 2, |_| true, 2, 1, 1, 100),
        ("platinum-up", 1, |q| q == 100, 1, 100, 2, 1),
    ]
}

fn converter_charge_matches(before: i64, after: Option<i64>, retirement: bool) -> bool {
    if retirement {
        before == 1 && after.is_none()
    } else {
        before > 0 && before.checked_sub(1) == after
    }
}

fn yell_matrix_matches(
    refused: oteryn_dev_client::ChatDisposition,
    accepted: oteryn_dev_client::ChatDisposition,
) -> bool {
    use oteryn_dev_client::ChatDisposition;
    refused == ChatDisposition::LevelTooLow && accepted == ChatDisposition::Ok
}

fn carried_coin(
    client: &DevClientSession,
    definition: std::num::NonZeroU32,
    pick: fn(u32) -> bool,
) -> Option<oteryn_dev_client::ItemEntry> {
    client
        .inventory()?
        .entries
        .iter()
        .find(|item| item.item_definition_ref == definition && pick(item.count.get()))
        .copied()
}

async fn withdraw_denominations(
    row: &mut Row,
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    refs: &[std::num::NonZeroU32; 3],
    withdrawals: &[u64],
) -> TestResult {
    let (npc, mut banker, bank) = find_banker(row, client, world, npcs).await?;
    let amount: u64 = withdrawals.iter().sum();
    if full_gold::coin_value(client)? < amount.saturating_sub(bank) {
        let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
        let meta = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
        full_gold::earn(
            client,
            world,
            &meta.spawns,
            "E03",
            amount.saturating_sub(bank),
        )
        .await?;
        banker = approach_npc(client, world, npc).await?;
        speak(client, banker, "hi").await?;
    }
    let inventory = inventory_units(client)?;
    let carried = wealth(&inventory, refs);
    if carried > 0 {
        bank_exchange(
            client,
            banker,
            "deposit all",
            &format!("Do you want to deposit {carried} gold?"),
            &format!("You have deposited {carried} gold."),
        )
        .await?;
    }
    for value in withdrawals {
        bank_exchange(
            client,
            banker,
            &format!("withdraw {value} gold"),
            &format!("Do you want to withdraw {value} gold?"),
            &format!("You have withdrawn {value} gold."),
        )
        .await?;
    }
    settle(client, "denomination-funding", |now| {
        wealth(now, refs) == amount && non_coin(now, refs) == non_coin(&inventory, refs)
    })
    .await?;
    let observed = bank_balance(client, banker).await?;
    if bank
        .checked_add(carried)
        .and_then(|total| total.checked_sub(amount))
        != Some(observed)
    {
        return Err("E03 denomination funding bank debit did not match carried custody".into());
    }
    Ok(())
}

type ConverterState = BTreeMap<Vec<u8>, (String, i64)>;

async fn converter_state(clients: &SeamClients<'_>) -> TestResult<ConverterState> {
    use sqlx::Connection as _;
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut connection = sqlx::PgConnection::connect(clients.url).await?;
        let rows: Vec<(Vec<u8>, String, i64)> = sqlx::query_as(
            "SELECT uuid_send(i.item_instance_id),i.definition_production_key,COALESCE(t.charges,500)::bigint              FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id)              LEFT JOIN game_item_timed_states t USING(item_instance_id)              WHERE e.character_id=encode($1,'hex')::uuid AND e.world_id=encode($2,'hex')::uuid              AND i.lifecycle=1 AND i.definition_production_key IN              ('oteryn:item.tibia.i23722','oteryn:item.tibia.i25719','oteryn:item.tibia.i28525','oteryn:item.tibia.i28526')"
        ).bind(clients.characters[1].as_slice()).bind(clients.world.as_slice())
            .fetch_all(&mut connection).await?;
        Ok(rows.into_iter().map(|(id,key,charges)| (id,(key,charges))).collect())
    }).await?
}

async fn converter_retired(clients: &SeamClients<'_>, id: &[u8]) -> TestResult<bool> {
    use sqlx::Connection as _;
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut connection = sqlx::PgConnection::connect(clients.url).await?;
        Ok(sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM game_item_instances
             WHERE item_instance_id = encode($1,'hex')::uuid
               AND world_id = encode($2,'hex')::uuid AND lifecycle = 2 AND quantity = 0)",
        )
        .bind(id)
        .bind(clients.world.as_slice())
        .fetch_one(&mut connection)
        .await?)
    })
    .await?
}

async fn converter_checks(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    refs: &[std::num::NonZeroU32; 3],
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    let defs = provisioning::item_index()?;
    let temporary = provisioning::item_reference(&defs, 25719)?;
    let initial = converter_state(clients).await?;
    let mut converters = initial
        .iter()
        .filter(|(_, (key, _))| key == "oteryn:item.tibia.i25719");
    let (id, &(_, mut charges)) = converters
        .next()
        .ok_or("E03 temporary converter is absent from observed custody")?;
    if converters.next().is_some() {
        return Err("E03 needs one distinguishable temporary converter".into());
    }
    let mut proved_charge = false;
    loop {
        let tool = carried_coin(client, temporary, |_| true)
            .ok_or("E03 temporary converter handle absent")?;
        let before = coin_counts(&inventory_units(client)?, refs);
        let (target, expected) = if let Some(item) = carried_coin(client, refs[0], |q| q == 100) {
            (item, [before[0] - 100, before[1] + 1, before[2]])
        } else {
            let item = carried_coin(client, refs[1], |q| q < 100)
                .ok_or("E03 converter needs a 100 gold or sub-100 platinum stack")?;
            (item, [before[0] + 100, before[1] - 1, before[2]])
        };
        full_e2ef7::use_on(client, tool.handle, target.handle).await?;
        settle(client, "converter-coin-delta", |now| {
            coin_counts(now, refs) == expected
        })
        .await?;
        let after = converter_state(clients).await?;
        let remaining = after.get(id).map(|(_, charges)| *charges);
        if charges == 1 && !converter_retired(clients, id).await? {
            return Err("E03 last charge did not retire the exact durable converter".into());
        }
        if !converter_charge_matches(charges, remaining, charges == 1) {
            return Err(
                format!("E03 converter charge custody mismatch: {charges}->{remaining:?}").into(),
            );
        }
        if let Some(remaining) = remaining {
            if !proved_charge {
                row.pass(
                    "converter-charge",
                    format!("exact-one-charge:{charges}->{remaining}"),
                );
                proved_charge = true;
            }
            charges = remaining;
        } else {
            if carried_coin(client, temporary, |_| true).is_some() {
                return Err("E03 spent temporary converter retained a carried handle".into());
            }
            if !proved_charge {
                row.pass("converter-charge", "one-charge-consumed-by-last-conversion");
            }
            row.pass(
                "converter-retirement",
                "last-charge-consumed-and-converter-removed",
            );
            break;
        }
        beat(row);
    }
    let inactive = provisioning::item_reference(&defs, 28525)?;
    let active = provisioning::item_reference(&defs, 28526)?;
    // Install a full gold stack while inactive, so the next tick has an exact input.
    if carried_coin(client, refs[0], |q| q == 100).is_none() {
        let coins = carried_coin(client, refs[1], |q| q < 100)
            .ok_or("E03 magic converter setup needs a platinum unit for gold change")?;
        let before = coin_counts(&inventory_units(client)?, refs);
        let changed = client.use_item(coins.handle).await?;
        if changed.disposition != UseDisposition::Committed {
            return Err(
                format!("E03 magic-converter input preparation refused: {changed:?}").into(),
            );
        }
        settle(client, "magic-input", |now| {
            coin_counts(now, refs) == [before[0] + 100, before[1] - 1, before[2]]
        })
        .await?;
    }
    let tool =
        carried_coin(client, inactive, |_| true).ok_or("E03 inactive magic converter missing")?;
    let durable = converter_state(clients).await?;
    let mut candidates = durable
        .iter()
        .filter(|(_, (key, _))| key == "oteryn:item.tibia.i28525");
    let (id, (_, before_charges)) = candidates
        .next()
        .ok_or("E03 inactive magic converter custody absent")?;
    if *before_charges < 2 || candidates.next().is_some() {
        return Err("E03 needs one magic converter with at least two charges".into());
    }
    let before = coin_counts(&inventory_units(client)?, refs);
    let toggled = client.use_item(tool.handle).await?;
    if toggled.disposition != UseDisposition::Committed {
        return Err(format!("E03 magic converter activation refused: {toggled:?}").into());
    }
    settle(client, "magic-active", |now| {
        now.get(&active.get()) == Some(&1)
    })
    .await?;
    row.pass("magic-toggle", "ordinary-use-switched-exact-magic-form-on");
    let expected = [before[0] - 100, before[1] + 1, before[2]];
    settle(client, "magic-tick", |now| {
        coin_counts(now, refs) == expected
    })
    .await?;
    let after = converter_state(clients).await?;
    if !after.get(id).is_some_and(|(key, charges)| {
        key == "oteryn:item.tibia.i28526"
            && converter_charge_matches(*before_charges, Some(*charges), false)
    }) {
        return Err(
            "E03 magic tick did not consume exactly one charge of the activated instance".into(),
        );
    }
    row.pass("magic-tick", "automatic-coin-upgrade-spent-one-charge");
    let current =
        carried_coin(client, active, |_| true).ok_or("E03 active converter handle absent")?;
    let off = client.use_item(current.handle).await?;
    if off.disposition != UseDisposition::Committed {
        return Err(format!("E03 magic converter deactivation refused: {off:?}").into());
    }
    settle(client, "magic-off", |now| {
        now.get(&inactive.get()) == Some(&1)
            && !now.contains_key(&active.get())
            && coin_counts(now, refs) == expected
    })
    .await?;
    let off_state = converter_state(clients).await?;
    if !off_state.get(id).is_some_and(|(key, charges)| {
        key == "oteryn:item.tibia.i28525"
            && converter_charge_matches(*before_charges, Some(*charges), false)
    }) {
        return Err("E03 deactivation changed magic converter charges or instance".into());
    }
    Ok(())
}

async fn make_gold_lot(
    client: &mut DevClientSession,
    refs: &[std::num::NonZeroU32; 3],
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    let platinum = carried_coin(client, refs[1], |quantity| quantity == 1)
        .ok_or("E03 funding withdrawal did not supply one platinum")?;
    let before = coin_counts(&inventory_units(client)?, refs);
    let change = client.use_item(platinum.handle).await?;
    if change.disposition != UseDisposition::Committed {
        return Err(format!("E03 assembling the gold lot: {change:?}").into());
    }
    settle(client, "gold-lot-change", |now| {
        coin_counts(now, refs) == [before[0] + 100, before[1] - 1, before[2]]
    })
    .await?;
    if carried_coin(client, refs[0], |count| count == 100).is_none() {
        return Err("E03 coin change never delivered the exact gold lot".into());
    }
    Ok(())
}

async fn provision_converters(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    row: &mut Row,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    refs: &[std::num::NonZeroU32; 3],
) -> TestResult {
    let defs = provisioning::item_index()?;
    let temporary = provisioning::item_reference(&defs, 25719)?;
    let magic = provisioning::item_reference(&defs, 28525)?;
    let active = provisioning::item_reference(&defs, 28526)?;
    if let Some(tool) = carried_coin(client, active, |_| true) {
        let reply = client.use_item(tool.handle).await?;
        if reply.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
            return Err(format!("E03 active converter could not be deactivated: {reply:?}").into());
        }
        settle(client, "converter-fixture-normalize", |now| {
            now.get(&magic.get()) == Some(&1) && !now.contains_key(&active.get())
        })
        .await?;
    }
    let durable = converter_state(clients).await?;
    if let Some((id, _)) = durable
        .iter()
        .find(|(_, (key, charges))| key == "oteryn:item.tibia.i28525" && *charges == 1)
    {
        withdraw_denominations(row, client, world, npcs, refs, &[100]).await?;
        make_gold_lot(client, refs).await?;
        let tool = carried_coin(client, magic, |_| true)
            .ok_or("E03 spent Magic converter handle absent")?;
        let before = coin_counts(&inventory_units(client)?, refs);
        let activated = client.use_item(tool.handle).await?;
        if activated.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
        {
            return Err(format!("E03 final Magic charge activation refused: {activated:?}").into());
        }
        settle(client, "magic-final-charge", |now| {
            coin_counts(now, refs) == [before[0] - 100, before[1] + 1, before[2]]
                && !now.contains_key(&magic.get())
                && !now.contains_key(&active.get())
        })
        .await?;
        if !converter_retired(clients, id).await? {
            return Err("E03 final Magic charge did not retire the exact converter".into());
        }
    }
    if carried_coin(client, temporary, |_| true).is_some()
        && carried_coin(client, magic, |_| true).is_some()
    {
        return Ok(());
    }
    let socket = required("NODE_BOOT_CONTROL_SOCKET")?;
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/qualification/node_boot/provision.py");
    let task = tokio::task::spawn_blocking(move || {
        std::process::Command::new("sudo")
            .args(["-n", "python3"])
            .arg(script)
            .args([
                "--socket",
                &socket,
                "--session",
                &session,
                "--profile",
                "coin-converters",
            ])
            .status()
            .map_err(|error| error.to_string())
    });
    if !wait_fixture_command(task, async || {
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
        Ok(())
    })
    .await?
    .success()
    {
        return Err("E03 audited converter fixture refused".into());
    }
    relog(client, clients).await?;
    Ok(())
}

async fn coins(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    let refs = coin_refs()?;
    let mut failures: Vec<String> = Vec::new();
    for (action, used, pick, spend, spent, gain, gained) in coin_steps() {
        if carried_coin(client, refs[used], pick).is_none() {
            let funding: &[u64] = match action {
                "gold-up" => &[100],
                "platinum-down" => &[100],
                "crystal-down" => &[10_000],
                _ => &[],
            };
            if !funding.is_empty()
                && let Err(error) =
                    withdraw_denominations(row, client, world, npcs, &refs, funding).await
            {
                failures.push(format!("{action}: denomination funding failed: {error}"));
            }
        }
        if action == "gold-up" && carried_coin(client, refs[0], pick).is_none() {
            make_gold_lot(client, &refs).await?;
        }
        let units = inventory_units(client)?;
        let before = coin_counts(&units, &refs);
        let item = carried_coin(client, refs[used], pick);
        let Some(item) = item else {
            failures.push(format!(
                "{action}: no eligible stack carried (coins gold/platinum/crystal={before:?}); coins can only come from real sales, loot and bank withdrawals"
            ));
            continue;
        };
        let outcome = client.use_item(item.handle).await?;
        if outcome.disposition != UseDisposition::Committed {
            failures.push(format!("{action}: USE refused: {:?}", outcome.disposition));
            continue;
        }
        let mut expected = before;
        expected[spend] -= spent;
        expected[gain] += gained;
        match settle(client, action, |now| coin_counts(now, &refs) == expected).await {
            Ok(_) => row.pass(action, format!("exact-coin-delta:{before:?}->{expected:?}")),
            Err(error) => failures.push(error.to_string()),
        }
        beat(row);
    }
    provision_converters(client, clients, row, world, npcs, &refs).await?;
    withdraw_denominations(row, client, world, npcs, &refs, &[100]).await?;
    make_gold_lot(client, &refs).await?;
    if let Err(error) = converter_checks(row, client, clients, &refs).await {
        failures.push(error.to_string());
    }
    let charges = converter_state(clients).await?;
    let carried = inventory_units(client)?;
    relog(client, clients).await?;
    if inventory_units(client)? != carried || converter_state(clients).await? != charges {
        failures.push(format!(
            "relog-coins-charges: carried custody changed across fresh admission: before={carried:?} after={:?}",
            inventory_units(client)?
        ));
    } else if row.passed.contains("converter-charge") && row.passed.contains("magic-tick") {
        row.pass(
            "relog-coins-charges",
            "fresh-admission-retained-coin-and-converter-charge-custody",
        );
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join(" | ").into())
    }
}

fn race_custody_matches(
    outcome: oteryn_protocol_oteryn::player_trade::ResultCode,
    unchanged: bool,
    swapped: bool,
    closed: bool,
) -> bool {
    use oteryn_protocol_oteryn::player_trade::ResultCode::*;
    closed
        && match outcome {
            Completed => swapped && !unchanged,
            Cancelled | Stale => unchanged && !swapped,
            _ => false,
        }
}

fn distinguishable_lots(a: (u32, u64), b: (u32, u64)) -> bool {
    a != b
}

fn distinct_trade_items(
    giver: &DevClientSession,
    recipient: &DevClientSession,
) -> TestResult<(oteryn_dev_client::ItemEntry, oteryn_dev_client::ItemEntry)> {
    let mine = &giver
        .inventory()
        .ok_or("trade giver inventory absent")?
        .entries;
    let theirs = &recipient
        .inventory()
        .ok_or("trade recipient inventory absent")?
        .entries;
    for a in mine {
        if let Some(b) = theirs.iter().find(|b| {
            distinguishable_lots(
                (a.item_definition_ref.get(), u64::from(a.count.get())),
                (b.item_definition_ref.get(), u64::from(b.count.get())),
            )
        }) {
            return Ok((*a, *b));
        }
    }
    Err("trade needs observably distinct carried lots".into())
}

// The inventory wire order is the durable backpack order (newest ordinal first).
// Check the entire view before using an index to bind a session-local handle.
async fn backpack_instances(
    clients: &SeamClients<'_>,
    character: usize,
    client: &DevClientSession,
) -> TestResult<Vec<Vec<u8>>> {
    use sqlx::Connection as _;
    let inventory = client
        .inventory()
        .ok_or("durable backpack proof lacks inventory")?;
    let index = provisioning::item_index()?;
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut connection = sqlx::PgConnection::connect(clients.url).await?;
        let rows: Vec<(Vec<u8>, String, String, i64)> = sqlx::query_as(
            "SELECT uuid_send(e.item_instance_id), i.definition_production_key,
                    i.definition_revision_ref, i.quantity
             FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id,world_id)
             WHERE e.character_id = encode($1,'hex')::uuid AND e.world_id = encode($2,'hex')::uuid
                   AND i.lifecycle = 1 ORDER BY e.placement_ordinal DESC",
        )
        .bind(clients.characters[character].as_slice())
        .bind(clients.world.as_slice())
        .fetch_all(&mut connection)
        .await?;
        if rows.len() != inventory.entries.len() || rows.iter().zip(&inventory.entries).any(
            |((_, key, revision, quantity), entry)| {
                index.definition_ref("Item", key, revision) != Some(entry.item_definition_ref)
                    || *quantity != i64::from(entry.count.get())
            },
        ) {
            return Err("durable backpack order differs from the wire view".into());
        }
        Ok(rows.into_iter().map(|(id, _, _, _)| id).collect())
    }).await?
}

fn offered_instance<'a>(
    client: &DevClientSession,
    instances: &'a [Vec<u8>],
    item: oteryn_dev_client::ItemEntry,
) -> TestResult<&'a Vec<u8>> {
    client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .position(|entry| entry.handle == item.handle)
        })
        .and_then(|index| instances.get(index))
        .ok_or_else(|| "offered handle lacks a durable backpack identity".into())
}

fn exact_trade_custody(
    before: [&[Vec<u8>]; 2],
    after: [&[Vec<u8>]; 2],
    offered: [&Vec<u8>; 2],
    completed: bool,
) -> bool {
    (0..2).all(|side| {
        let mut expected: BTreeSet<_> = before[side].iter().collect();
        if completed {
            expected.remove(offered[side]);
            expected.insert(offered[1 - side]);
        }
        expected == after[side].iter().collect()
    })
}

async fn trade(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
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
    let item = client
        .inventory()
        .and_then(|i| i.entries.first())
        .copied()
        .ok_or("no direct-backpack leaf for real trade;starter backpack itself is not a leaf")?;
    let mut peer = while_serving(client, connect_character(clients, 0x81, 0)).await?;
    let result: TestResult = async {
        let other = tile(peer.world_spatial().actor_position)?;
        while_serving(&mut peer, approach_peer_tile(client, other, world)).await?;
        let command = runtime::PlayerTradeCommand::new(runtime::player_trade::Intent::Offer {
            partner: own(&peer)?,
            item: item.handle.get(),
        })
        .ok_or("trade offer encoding")?;
        let (reply, offer_delta) = tokio::join!(
            runtime::send(client.session_mut(), &command),
            peer.session_mut().read_gated_delta(26, 1),
        );
        let reply = reply?;
        if reply.result != runtime::player_trade::ResultCode::Offered {
            return Err(format!("trade request: {:?}", reply.result).into());
        }
        row.pass("request", "real-peer-offer-opened");
        offer_delta?;
        let id = reply.state.trade_id.ok_or("offer has no trade id")?;
        let command = runtime::PlayerTradeCommand::new(runtime::player_trade::Intent::Look {
            trade_id: id,
            counter_offer: false,
        })
        .ok_or("trade look encoding")?;
        let reply = while_serving(&mut peer, runtime::send(client.session_mut(), &command)).await?;
        if reply.state.look.is_empty() {
            return Err("offered leaf look was empty".into());
        }
        row.pass("look", "offered-leaf-description");
        let command = runtime::PlayerTradeCommand::new(runtime::player_trade::Intent::Cancel)
            .ok_or("trade cancel encoding")?;
        let reply = while_serving(&mut peer, runtime::send(client.session_mut(), &command)).await?;
        if reply.result != runtime::player_trade::ResultCode::Cancelled || reply.state.phase != 4 {
            return Err("active trade cancellation did not close escrow".into());
        }
        row.pass("cancel", "active-offer-closed");
        let (item, peer_item) = distinct_trade_items(client, &peer)?;
        let mine = inventory_units(client)?;
        let theirs = inventory_units(&peer)?;
        // Round two: both players offer, both accept, custody swaps exactly.
        let offer = runtime::PlayerTradeCommand::new(runtime::player_trade::Intent::Offer {
            partner: own(&peer)?,
            item: item.handle.get(),
        })
        .ok_or("second offer encoding")?;
        let reply = while_serving(&mut peer, runtime::send(client.session_mut(), &offer)).await?;
        if reply.result != runtime::player_trade::ResultCode::Offered {
            return Err(format!("second offer: {:?}", reply.result).into());
        }
        let id = reply.state.trade_id.ok_or("second offer has no trade id")?;
        let counter = runtime::PlayerTradeCommand::new(runtime::player_trade::Intent::Offer {
            partner: own(client)?,
            item: peer_item.handle.get(),
        })
        .ok_or("counter offer encoding")?;
        let reply = while_serving(client, runtime::send(peer.session_mut(), &counter)).await?;
        if reply.result != runtime::player_trade::ResultCode::Offered && reply.result != runtime::player_trade::ResultCode::Accepted {
            return Err(format!("peer counter-offer: {:?}", reply.result).into());
        }
        let accept = runtime::PlayerTradeCommand::new(runtime::player_trade::Intent::Accept { trade_id: id })
            .ok_or("accept encoding")?;
        let first = while_serving(&mut peer, runtime::send(client.session_mut(), &accept)).await?;
        if first.result != runtime::player_trade::ResultCode::Accepted {
            return Err(format!("first accept: {:?}", first.result).into());
        }
        let second = while_serving(client, runtime::send(peer.session_mut(), &accept)).await?;
        if second.result != runtime::player_trade::ResultCode::Completed {
            return Err(format!("second accept: {:?}", second.result).into());
        }
        let swap = |own: &BTreeMap<u32, u64>, given: u32, got: u32, n_given: u64, n_got: u64| {
            let mut next = own.clone();
            let gave = next.entry(given).or_default();
            *gave = gave.saturating_sub(n_given);
            if *gave == 0 {
                next.remove(&given);
            }
            *next.entry(got).or_default() += n_got;
            next
        };
        let a = (item.item_definition_ref.get(), u64::from(item.count.get()));
        let b = (peer_item.item_definition_ref.get(), u64::from(peer_item.count.get()));
        let client_after = swap(&mine, a.0, b.0, a.1, b.1);
        let peer_after = swap(&theirs, b.0, a.0, b.1, a.1);
        while_serving(&mut peer, settle(client, "trade-accept", |now| *now == client_after)).await?;
        while_serving(client, settle(&mut peer, "trade-accept-peer", |now| *now == peer_after)).await?;
        if [(&*client, item.handle), (&peer, peer_item.handle)].iter().any(|(participant, handle)| {
            participant.inventory().is_none_or(|inventory| inventory.entries.iter()
                .any(|entry| entry.handle == *handle))
        }) {
            return Err("completed trade retained an offered instance with its previous owner".into());
        }
        row.pass("accept", "both-accepts-swapped-custody-exactly");
        beat(row);
        // Round three races the final accept against owner logout.
        let (item, peer_item) = distinct_trade_items(client, &peer)?;
        let before_client = inventory_units(client)?;
        let before_peer = inventory_units(&peer)?;
        let client_instances = backpack_instances(clients, 1, client).await?;
        let peer_instances = backpack_instances(clients, 0, &peer).await?;
        let offered = [offered_instance(client, &client_instances, item)?,
            offered_instance(&peer, &peer_instances, peer_item)?];
        let offer = runtime::PlayerTradeCommand::new(runtime::player_trade::Intent::Offer {
            partner: own(&peer)?,
            item: item.handle.get(),
        })
        .ok_or("race offer encoding")?;
        let reply = while_serving(&mut peer, runtime::send(client.session_mut(), &offer)).await?;
        if reply.result != runtime::player_trade::ResultCode::Offered {
            return Err(format!("race offer: {:?}", reply.result).into());
        }
        let id = reply.state.trade_id.ok_or("race offer has no trade id")?;
        let counter = runtime::PlayerTradeCommand::new(runtime::player_trade::Intent::Offer {
            partner: own(client)?,
            item: peer_item.handle.get(),
        })
        .ok_or("race counter encoding")?;
        let counter = while_serving(client, runtime::send(peer.session_mut(), &counter)).await?;
        if counter.state.trade_id != Some(id)
            || !matches!(counter.result, runtime::player_trade::ResultCode::Offered | runtime::player_trade::ResultCode::Accepted)
        {
            return Err(format!("race counter-offer did not enter the offered trade: {counter:?}").into());
        }
        let accept = runtime::PlayerTradeCommand::new(runtime::player_trade::Intent::Accept {
            trade_id: id,
        })
        .ok_or("race accept encoding")?;
        let first = while_serving(&mut peer, runtime::send(client.session_mut(), &accept)).await?;
        if first.result != runtime::player_trade::ResultCode::Accepted {
            return Err(format!("owner accept before the race: {:?}", first.result).into());
        }
        let (accepted, (logout_result, logout_text, attempts, busy_timed_out)) = {
            let mut logout = std::pin::pin!(super::logout_retrying_busy(client));
            let first = {
                let mut accepting = std::pin::pin!(runtime::send(peer.session_mut(), &accept));
                tokio::select! {
                    accepted = &mut accepting => Ok(accepted),
                    logged_out = &mut logout => Err((logged_out, accepting.await)),
                }
            };
            match first {
                Ok(accepted) => (accepted, while_serving(&mut peer, logout).await?),
                Err((logged_out, accepted)) => (accepted, logged_out?),
            }
        };
        if logout_result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Err(format!("race logout refused: {logout_text}; attempts={attempts}; busy_timed_out={busy_timed_out}; peer accept={:?}", accepted.map(|r| r.result)).into());
        }
        let outcome = accepted?.result;
        *client = while_serving(&mut peer, connect_character(clients, 0x85, 1)).await?;
        let a = (item.item_definition_ref.get(), u64::from(item.count.get()));
        let b = (peer_item.item_definition_ref.get(), u64::from(peer_item.count.get()));
        let swapped_client = swap(&before_client, a.0, b.0, a.1, b.1);
        let swapped_peer = swap(&before_peer, b.0, a.0, b.1, a.1);
        let now_client = while_serving(&mut peer, settle(client, "race-relog", |now| {
            *now == before_client || *now == swapped_client
        }))
        .await?;
        let completed = now_client == swapped_client && now_client != before_client;
        let (expected_peer, side) = if completed {
            (&swapped_peer, "completed-swap")
        } else {
            (&before_peer, "cancelled-unchanged")
        };
        while_serving(client, settle(&mut peer, "race-peer", |now| now == expected_peer)).await?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        let closed = loop {
            let state = peer.session_mut().player_trade_state();
            if state.as_ref().is_some_and(|state| {
                state.trade_id == Some(id) && state.phase == 4
                    && if completed { state.reason == 1 } else { matches!(state.reason, 2 | 4) }
            }) {
                break true;
            }
            if tokio::time::Instant::now() >= deadline {
                break false;
            }
            while_serving(client, peer.service_liveness(Duration::from_millis(20))).await?;
            peer.take_events();
        };
        let client_after = backpack_instances(clients, 1, client).await?;
        let peer_after = backpack_instances(clients, 0, &peer).await?;
        if !exact_trade_custody([&client_instances, &peer_instances], [&client_after, &peer_after], offered, completed)
            || !race_custody_matches(outcome, !completed, completed, closed) {
            return Err(format!("race disposition/custody mismatch: {outcome:?} {side}; closed={closed}").into());
        }
        row.pass("logout-race", "partner-accept-and-logout-resolved-the-same-trade");
        row.pass(
            "relog-custody",
            format!("fresh-admission-custody-exactly-{side}-on-both-sides"),
        );
        Ok(())
    }
    .await;
    let (cleanup_result, cleanup, attempts, busy_timed_out) =
        match while_serving(client, super::logout_retrying_busy(&mut peer)).await {
            Ok(cleanup) => cleanup,
            Err(error) => return result.and(Err(error)),
        };
    if cleanup_result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("trade peer cleanup logout refused: {cleanup}; attempts={attempts}; busy_timed_out={busy_timed_out}; journey={result:?}").into());
    }
    result
}

async fn market(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    full_e2ef4_f4e05::run(row, client, clients, world).await
}

async fn depot_call(
    client: &mut DevClientSession,
    town: Option<u16>,
    action: oteryn_protocol_oteryn::depot::DepotAction,
) -> TestResult<oteryn_protocol_oteryn::depot::DepotReply> {
    depot_page(client, town, 0, action).await
}

async fn depot_page(
    client: &mut DevClientSession,
    town: Option<u16>,
    page: u32,
    action: oteryn_protocol_oteryn::depot::DepotAction,
) -> TestResult<oteryn_protocol_oteryn::depot::DepotReply> {
    use oteryn_protocol_oteryn::depot;
    let payload = depot::encode_depot_intent(depot::DepotIntent {
        town,
        box_number: 1,
        page,
        action,
    })
    .map_err(|e| format!("depot intent: {e:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            depot::CAPABILITY_DEPOT_V1,
            depot::COMMAND_TYPE_DEPOT_INTENT,
            &payload,
        )
        .await?;
    let depot_reply =
        depot::decode_depot_reply(&reply.payload).map_err(|e| format!("depot reply: {e:?}"))?;
    let success = matches!(
        depot_reply.outcome,
        depot::DepotOutcome::Opened | depot::DepotOutcome::Moved
    );
    if success && !full_e2ef4_f4e07::successful_storage_reply(reply.status, success) {
        return Err(format!(
            "depot success payload on {:?}: {depot_reply:?}",
            reply.status
        )
        .into());
    }
    Ok(depot_reply)
}

async fn storage(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    full_e2ef4_f4e07::storage(row, client, clients, world).await
}

#[path = "full_e2ef4_f4s01.rs"]
mod full_e2ef4_f4s01;

async fn party(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    full_e2ef4_f4s01::run(row, client, clients, world).await
}

#[path = "full_e2ef4_f4s03.rs"]
mod full_e2ef4_f4s03;

async fn guild(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult {
    full_e2ef4_f4s03::run(row, client, clients, world, npcs).await
}

// Every message a session received, so a refusal or warning is read from the wire.
fn notices(session: &mut DevClientSession) -> Vec<String> {
    use oteryn_dev_client::SessionEvent;
    use oteryn_protocol_oteryn::tutorial_notices::TutorialNotice;
    session
        .take_events()
        .into_iter()
        .filter_map(|event| match event {
            SessionEvent::TutorialNotice(delta) => match delta.value {
                TutorialNotice::StatusText(text) => Some(text),
                TutorialNotice::Hint(_) => None,
            },
            _ => None,
        })
        .collect()
}

async fn logout(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    let failures = while_serving(client, logout_peer_checks(row, clients, world)).await?;
    logout_idle(row, client, clients, world, failures).await
}

fn saved_logout_position(start: TilePos, saved: TilePos, restored: TilePos) -> bool {
    saved != start && saved == restored
}

async fn logout_peer_checks(
    row: &mut Row,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult<Vec<String>> {
    let mut peer = connect_character(clients, 0x83, 0).await?;
    let start = tile(peer.world_spatial().actor_position)?;
    let path = route(world, start, |at| {
        at != start
            && at != TOWN
            && world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|tile| tile.protection_zone())
    });
    let Some(path) = path else {
        let (_, cleanup, attempts, busy_timed_out) = super::logout_retrying_busy(&mut peer).await?;
        return Err(format!("logout peer has no reachable distinct authored PZ tile: cleanup={cleanup}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
    };
    walk(&mut peer, world, &path).await?;
    let position = tile(peer.world_spatial().actor_position)?;
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
    let retained =
        saved_logout_position(start, position, tile(peer.world_spatial().actor_position)?)
            && peer.join_snapshot().game_session_id != session;
    let (cleanup_result, cleanup, attempts, busy_timed_out) =
        super::logout_retrying_busy(&mut peer).await?;
    if !retained || cleanup_result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("fresh-login-position-or-cleanup-mismatch: retained={retained}; cleanup={cleanup}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
    }
    row.pass("relog-position", "fresh-session-retained-saved-position");
    let mut failures: Vec<String> = Vec::new();
    // Fight refusal: a real rat fight, then the native LOGOUT is refused as InFight.
    let fight: TestResult = async {
        use oteryn_protocol_oteryn::{attack::AttackIntentDisposition, logout::LogoutResult};
        let bytes = std::fs::read(std::env::var("NODE_BOOT_WORLD_BUNDLE")?)?;
        let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
        let mut fighter = connect_character(clients, 0x8a, 0).await?;
        let journey: TestResult = async {
            super::travel_to_spawn(&mut fighter, world, &metadata.spawns, "oteryn:creature.rat")
                .await?;
            let here = fighter.world_spatial().actor_position;
            let target = fighter
                .world_entities()
                .and_then(|entities| {
                    entities.iter().find(|entry| {
                        entry.kind == oteryn_dev_client::EntityKind::Creature
                            && entry.position.floor == here.floor
                            && entry
                                .position
                                .x
                                .abs_diff(here.x)
                                .max(entry.position.y.abs_diff(here.y))
                                <= 1
                    })
                })
                .copied()
                .ok_or("no adjacent rat after walking to the spawn")?;
            fighter.take_events();
            let attack = fighter
                .session_mut()
                .attack_target(Some(&target.entity))
                .await?;
            if attack.disposition != AttackIntentDisposition::Ok {
                return Err(format!("attack refused: {attack:?}").into());
            }
            let refused = fighter.session_mut().logout().await?;
            let Some(LogoutResult::InFight { retry_after_ms }) = refused.result else {
                return Err(format!(
                    "logout during a fight was not refused as InFight: {refused:?}"
                )
                .into());
            };
            if retry_after_ms == 0 {
                return Err("InFight refusal carried no retry wait".into());
            }
            row.pass(
                "fight-refusal",
                format!("in-fight-logout-refused-retry-after-{retry_after_ms}ms"),
            );
            Ok(())
        }
        .await;
        // The retry path doubles as cleanup once the combat lock expires.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
        loop {
            beat(row);
            fighter.session_mut().attack_target(None).await?;
            fighter.service_liveness(Duration::from_millis(250)).await?;
            let reply = fighter.session_mut().logout().await?;
            match reply.result {
                Some(LogoutResult::Accepted) => break,
                Some(LogoutResult::InFight { .. } | LogoutResult::Busy)
                    if tokio::time::Instant::now() < deadline => {}
                _ => {
                    return Err(format!(
                        "fight peer cleanup logout: {reply:?}; journey={journey:?}"
                    )
                    .into());
                }
            }
        }
        journey
    }
    .await;
    if let Err(error) = fight {
        failures.push(format!("fight-refusal: {error}"));
    }
    beat(row);
    // NOLOGOUT: the same native LOGOUT on an authored no-logout tile is refused with the
    // server's own text.
    let nologout: TestResult = async {
        use oteryn_protocol_oteryn::{
            logout::LogoutResult, zones_nologout::NO_LOGOUT_MESSAGE,
        };
        let mut walker = connect_character(clients, 0x8b, 0).await?;
        let journey: TestResult = async {
            let path = route(world, tile(walker.world_spatial().actor_position)?, |at| {
                world
                    .base()
                    .tile(at.x, at.y, at.floor)
                    .is_some_and(|t| t.no_logout())
            })
            .ok_or("no authored NOLOGOUT tile is reachable from the town spawn")?;
            beat(row);
            walk(&mut walker, world, &path).await?;
            beat(row);
            walker.take_events();
            let reply = walker.session_mut().logout().await?;
            if reply.result == Some(LogoutResult::Accepted) {
                return Err(format!("logout on a NOLOGOUT tile was accepted: {reply:?}").into());
            }
            let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
            loop {
                if notices(&mut walker).iter().any(|t| t == NO_LOGOUT_MESSAGE) {
                    break;
                }
                if tokio::time::Instant::now() >= deadline {
                    return Err(format!("NOLOGOUT refusal ({reply:?}) never delivered {NO_LOGOUT_MESSAGE:?}").into());
                }
                progress();
                walker.service_liveness(Duration::from_millis(25)).await?;
            }
            row.pass("nologout", "no-logout-tile-refused-with-server-text");
            Ok(())
        }
        .await;
        let home = route(world, tile(walker.world_spatial().actor_position)?, |at| at == TOWN);
        if let Some(path) = home {
            walk(&mut walker, world, &path).await?;
        }
        let (cleanup_result, cleanup, attempts, busy_timed_out) =
            super::logout_retrying_busy(&mut walker).await?;
        if cleanup_result != Some(LogoutResult::Accepted) {
            return Err(format!("nologout peer cleanup refused: {cleanup}; attempts={attempts}; busy_timed_out={busy_timed_out}; journey={journey:?}").into());
        }
        journey
    }
    .await;
    if let Err(error) = nologout {
        failures.push(format!("nologout: {error}"));
    }
    beat(row);
    Ok(failures)
}

fn idle_removal_evidence(
    warned: Option<Duration>,
    removed: Duration,
    admission: Duration,
    eof: bool,
    released: bool,
) -> bool {
    eof && released
        && warned.is_some_and(|at| {
            (Duration::from_secs(900)..=admission + Duration::from_secs(910)).contains(&at)
        })
        && (Duration::from_secs(960)..=admission + Duration::from_secs(980)).contains(&removed)
}

fn idle_transport_closed(error: &oteryn_dev_client::DevClientError) -> bool {
    use oteryn_dev_client::{DevClientError, SessionError};
    match error {
        DevClientError::Context { source, .. } => idle_transport_closed(source),
        DevClientError::Session(SessionError::Io(error)) => {
            error.kind() == std::io::ErrorKind::UnexpectedEof
        }
        _ => false,
    }
}

async fn idle_session_released(
    clients: &SeamClients<'_>,
    session: oteryn_protocol_oteryn::GameSessionId,
) -> TestResult<bool> {
    use sqlx::Connection as _;
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut connection = sqlx::PgConnection::connect(clients.url).await?;
        let released = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions              WHERE game_session_id=encode($1,'hex')::uuid AND session_state=3              AND control_loss_epoch IS NULL)"
        )
        .bind(session.as_bytes().as_slice())
        .fetch_one(&mut connection)
        .await?;
        Ok(released)
    }).await?
}

async fn logout_idle(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    mut failures: Vec<String>,
) -> TestResult {
    // Idle: the server's idle policy runs on the real monotonic clock (warning at 15 minutes,
    // removal at 16), so this is a real wait. The peer stays silent; the main client acts once
    // at 14 minutes and must not be warned at 15.
    let idle: TestResult = async {
        let admission_started = tokio::time::Instant::now();
        let mut silent = while_serving(client, connect_character(clients, 0x8c, 0)).await?;
        let admission = admission_started.elapsed();
        let silent_session = silent.join_snapshot().game_session_id;
        let start = tokio::time::Instant::now();
        let mut warned_at = None;
        let mut warned_actor = false;
        let mut removed_at = None;
        let mut stepped = false;
        let mut anchored = false;
        while start.elapsed() < Duration::from_secs(1000) && removed_at.is_none() {
            if !anchored || (!stepped && start.elapsed() >= Duration::from_secs(840)) {
                let from = tile(client.world_spatial().actor_position)?;
                let path = route(world, from, |at| {
                    at != from
                        && at.floor == from.floor
                        && at.x.abs_diff(from.x).max(at.y.abs_diff(from.y)) <= 1
                })
                .ok_or("no adjacent tile for the idle-reset step")?;
                while_serving(&mut silent, walk(client, world, &path)).await?;
                if anchored {
                    stepped = true;
                }
                anchored = true;
            }
            let (silent_liveness, client_liveness) = tokio::join!(
                silent.service_liveness(Duration::from_secs(1)),
                client.service_liveness(Duration::from_secs(1)),
            );
            client_liveness?;
            if notices(client)
                .iter()
                .any(|t| t.starts_with("There was no variation in your behaviour"))
            {
                warned_actor = true;
            }
            match silent_liveness {
                Ok(_) => {
                    if warned_at.is_none()
                        && notices(&mut silent).iter().any(|t| {
                            t.starts_with("There was no variation in your behaviour for 15 minutes")
                        })
                    {
                        warned_at = Some(admission_started.elapsed());
                    }
                }
                Err(error) => {
                    let at = admission_started.elapsed();
                    let eof = idle_transport_closed(&error);
                    let released = eof && while_serving(client, idle_session_released(clients, silent_session)).await?;
                    if !idle_removal_evidence(warned_at, at, admission, eof, released)
                        || start.elapsed() < Duration::from_secs(900)
                    {
                        return Err(format!("idle peer closed without timed terminal logout evidence: warning={warned_at:?} close={at:?} terminal={released}: {error}").into());
                    }
                    removed_at = Some(at);
                }
            }
            beat(row);
        }
        if removed_at.is_none() {
            let (done, text, ..) =
                while_serving(client, super::logout_retrying_busy(&mut silent)).await?;
            if done != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
                return Err(format!(
                    "silent peer was never removed and its logout was refused: {text}"
                )
                .into());
            }
        }
        let mut errors: Vec<String> = Vec::new();
        match warned_at {
            Some(at) if (Duration::from_secs(900)..=admission + Duration::from_secs(910)).contains(&at) =>
                row.pass("idle-warning", format!("warned-at-{}s", at.as_secs())),
            _ => {
                errors.push("idle-warning: the silent peer received no 15-minute warning".into())
            }
        }
        match removed_at {
            Some(at) => {
                let reconnect = while_serving(client, async {
                    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
                    loop {
                        progress();
                        match tokio::time::timeout_at(deadline, connect_character(clients, 0x8d, 0)).await? {
                            Ok(mut again) => {
                                let id = again.join_snapshot().game_session_id;
                                let (done, text, ..) =
                                    super::logout_retrying_busy(&mut again).await?;
                                if done
                                    != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
                                {
                                    break Err(
                                        format!("reconnected peer logout refused: {text}").into()
                                    );
                                }
                                break Ok::<_, Box<dyn std::error::Error>>(id);
                            }
                            Err(error) if tokio::time::Instant::now() >= deadline => {
                                break Err(error);
                            }
                            Err(_) => tokio::time::sleep(Duration::from_millis(500)).await,
                        }
                    }
                })
                .await;
                match reconnect {
                    Ok(session) if session != silent_session => row.pass(
                        "idle-removal",
                        format!("removed-at-{}s-and-character-released", at.as_secs()),
                    ),
                    Ok(_) => {
                        errors.push("idle-removal: reconnect reused the removed session".into())
                    }
                    Err(error) => errors.push(format!(
                        "idle-removal: character not released after removal: {error}"
                    )),
                }
            }
            None => errors.push(
                "idle-removal: the silent peer was still connected 1000s after its last command"
                    .into(),
            ),
        }
        if warned_actor {
            errors.push(
                "gameplay-resets-idle: the acting client was warned at 15 minutes anyway".into(),
            );
        } else if stepped && removed_at.is_some() {
            row.pass(
                "gameplay-resets-idle",
                "step-at-14-minutes-suppressed-warning",
            );
        } else {
            errors.push("gameplay-resets-idle: the comparison window did not complete".into());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join(" | ").into())
        }
    }
    .await;
    if let Err(error) = idle {
        failures.push(error.to_string());
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join(" | ").into())
    }
}

fn speech_matches(
    line: &oteryn_dev_client::ChatLine,
    sequence: u64,
    boundary: u64,
    speaker: oteryn_dev_client::ChatSpeaker,
    mode: oteryn_dev_client::ChatSpeechMode,
    text: &str,
) -> bool {
    sequence > boundary
        && matches!(line, oteryn_dev_client::ChatLine::Local {
            speaker: source, mode: seen, text: said, ..
        } if *source == speaker && *seen == mode && said == text)
}

async fn listen(
    listener: &mut DevClientSession,
    boundary: u64,
    speaker: oteryn_dev_client::ChatSpeaker,
    mode: oteryn_dev_client::ChatSpeechMode,
    text: &str,
    window: Duration,
) -> TestResult<bool> {
    let deadline = tokio::time::Instant::now() + window;
    loop {
        if listener
            .take_events()
            .into_iter()
            .any(|event| speech_event(event, boundary, speaker, mode, text))
        {
            return Ok(true);
        }
        if tokio::time::Instant::now() >= deadline {
            return Ok(false);
        }
        progress();
        listener.service_liveness(Duration::from_millis(20)).await?;
    }
}

fn speech_event(
    event: oteryn_dev_client::SessionEvent,
    boundary: u64,
    speaker: oteryn_dev_client::ChatSpeaker,
    mode: oteryn_dev_client::ChatSpeechMode,
    text: &str,
) -> bool {
    matches!(event, oteryn_dev_client::SessionEvent::ChatLine(delta)
        if speech_matches(&delta.value, delta.server_sequence, boundary, speaker, mode, text))
}

async fn speech_cooldown(client: &mut DevClientSession, peer: &mut DevClientSession) -> TestResult {
    let duration = Duration::from_millis(1200);
    let (speaker, listener) = tokio::join!(
        client.service_liveness(duration),
        peer.service_liveness(duration),
    );
    speaker?;
    listener?;
    client.take_events();
    peer.take_events();
    Ok(())
}

fn far_speech_position(speaker: TilePos, listener: TilePos) -> bool {
    speaker.floor == listener.floor
        && speaker.x.abs_diff(listener.x) >= 10
        && speaker.y.abs_diff(listener.y) <= 2
}

async fn speech(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    use oteryn_dev_client::{ChatDisposition, ChatIntent, ChatSpeechMode};
    // Recipients are a real second player; every spoken line is read on the listener's own log.
    let mut peer = while_serving(client, connect_character(clients, 0x88, 0)).await?;
    let result: TestResult = async {
        let mut failures: Vec<String> = Vec::new();
        let other = tile(peer.world_spatial().actor_position)?;
        while_serving(&mut peer, approach_peer_tile(client, other, world)).await?;
        let window = Duration::from_secs(5);
        let client_level = while_serving(&mut peer, full_e2ef5::level(client)).await?;
        let peer_level = while_serving(client, full_e2ef5::level(&mut peer)).await?;
        let negative = ChatIntent::Say { mode: ChatSpeechMode::Yell, text: "E2EF4 ineligible yell".into() };
        let refused = if peer_level < 20 {
            while_serving(client, peer.chat(&negative)).await?.disposition
        } else if client_level < 20 {
            while_serving(&mut peer, client.chat(&negative)).await?.disposition
        } else {
            return Err("S18 needs one observed speaker below the free yell level".into());
        };
        if client_level < 20 {
            let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
            let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
            while_serving(&mut peer, full_e2ef6::earn_death_prerequisites(
                client, world, &metadata.spawns, 20, 0,
            )).await?;
            while_serving(&mut peer, approach_peer_tile(client, other, world)).await?;
        }
        let mut yell_client = None;
        let own = own(client)?;
        let speaker = oteryn_dev_client::ChatSpeaker {
            identity: own.identity,
            generation: own.generation.try_into()?,
        };
        for (action, mode) in [
            ("say", ChatSpeechMode::Say),
            ("whisper", ChatSpeechMode::Whisper),
            ("yell", ChatSpeechMode::Yell),
        ] {
            speech_cooldown(client, &mut peer).await?;
            let text = format!("E2EF4 {action}");
            let intent = ChatIntent::Say {
                mode,
                text: text.clone(),
            };
            let speaker_boundary = client.last_server_sequence();
            let listener_boundary = peer.last_server_sequence();
            let expected = if mode == ChatSpeechMode::Yell { text.to_uppercase() } else { text };
            let mut delivered = false;
            let mut witness = |event| {
                delivered |= speech_event(event, listener_boundary, speaker, mode, &expected);
            };
            let result = full_e2ef5::with_observer_events(&mut peer, async {
                Ok(client.chat(&intent).await?)
            }, &mut witness).await?;
            if mode == ChatSpeechMode::Yell {
                yell_client = Some(result.disposition);
            }
            if result.disposition != ChatDisposition::Ok {
                failures.push(format!(
                    "{action}: server refused the player's own {action}: disposition={:?} wait_seconds={}",
                    result.disposition, result.wait_seconds
                ));
                continue;
            }
            if !full_e2ef5::with_observer_events(
                &mut peer, listen(client, speaker_boundary, speaker, mode, &expected, window), witness,
            ).await? {
                failures.push(format!("{action}: own local line absent"));
            } else if !delivered && !while_serving(client, listen(&mut peer, listener_boundary, speaker, mode, &expected, window)).await? {
                failures.push(format!("{action}: adjacent real peer never heard it"));
            } else {
                row.pass(action, "speaker-and-adjacent-listener-exact-mode-and-text");
            }
            beat(row);
        }
        // Range: a whisper from a few tiles is obscured, and a say beyond eight tiles is unheard.
        let other = tile(peer.world_spatial().actor_position)?;
        let near = route(world, tile(client.world_spatial().actor_position)?, |at| {
            at.floor == other.floor
                && (3..=5).contains(&at.x.abs_diff(other.x))
                && at.y.abs_diff(other.y) <= 2
        });
        let mut range_proof = Vec::new();
        match near {
            Some(path) => {
                while_serving(&mut peer, walk(client, world, &path)).await?;
                speech_cooldown(client, &mut peer).await?;
                let intent = ChatIntent::Say {
                    mode: ChatSpeechMode::Whisper,
                    text: "E2EF4 range whisper".into(),
                };
                let listener_boundary = peer.last_server_sequence();
                let mut delivered = false;
                let result = full_e2ef5::with_observer_events(&mut peer, async {
                    Ok(client.chat(&intent).await?)
                }, |event| {
                    delivered |= speech_event(event, listener_boundary, speaker, ChatSpeechMode::Whisper, "pspsps");
                }).await?;
                if result.disposition != ChatDisposition::Ok {
                    failures.push(format!("range: whisper refused: {:?}", result.disposition));
                } else if delivered || while_serving(client, listen(&mut peer, listener_boundary, speaker, ChatSpeechMode::Whisper, "pspsps", window)).await? {
                    range_proof.push("whisper-obscured-at-distance");
                } else {
                    failures.push("range: distant whisper was not delivered obscured as pspsps".into());
                }
            }
            None => failures.push("range: no authored tile 3-5 columns from the peer".into()),
        }
        let far = route(world, tile(client.world_spatial().actor_position)?, |at| {
            far_speech_position(at, other)
        });
        match far {
            Some(path) => {
                while_serving(&mut peer, walk(client, world, &path)).await?;
                if !far_speech_position(
                    tile(client.world_spatial().actor_position)?,
                    tile(peer.world_spatial().actor_position)?,
                ) {
                    return Err("S18 walked endpoint did not establish far-say range".into());
                }
                speech_cooldown(client, &mut peer).await?;
                let text = "E2EF4 range say";
                let intent = ChatIntent::Say {
                    mode: ChatSpeechMode::Say,
                    text: text.into(),
                };
                let speaker_boundary = client.last_server_sequence();
                let listener_boundary = peer.last_server_sequence();
                let mut delivered = false;
                let mut witness = |event| {
                    delivered |= speech_event(event, listener_boundary, speaker, ChatSpeechMode::Say, text);
                };
                let result = full_e2ef5::with_observer_events(&mut peer, async {
                    Ok(client.chat(&intent).await?)
                }, &mut witness).await?;
                if result.disposition != ChatDisposition::Ok {
                    failures.push(format!("range: far say refused: {:?}", result.disposition));
                } else if !full_e2ef5::with_observer_events(&mut peer,
                    listen(client, speaker_boundary, speaker, ChatSpeechMode::Say, text, window), witness,
                ).await? {
                    failures.push("range: far say has no own local line".into());
                } else if delivered || while_serving(client, listen(&mut peer, listener_boundary, speaker, ChatSpeechMode::Say, text, Duration::from_secs(2))).await? {
                    failures.push("range: say beyond eight tiles was heard".into());
                } else {
                    range_proof.push("say-unheard-beyond-eight-tiles");
                }
            }
            None => failures.push("range: no authored tile 10 or more columns from the peer".into()),
        }
        if range_proof.len() == 2 {
            row.pass("range", range_proof.join("+"));
        }
        if yell_client.is_some_and(|accepted| yell_matrix_matches(refused, accepted))
            && row.passed.contains("yell")
        {
            row.pass("yell-level-premium", "underlevel-refusal-and-earned-eligible-yell-delivered");
        } else {
            failures.push(format!("yell-level-premium: ineligible={refused:?}; eligible={yell_client:?}"));
        }
        speech_cooldown(client, &mut peer).await?;
        let last = while_serving(&mut peer, async {
            let mut last = None;
            for index in 0..64 {
                progress();
                let result = client
                    .chat(&ChatIntent::Say {
                        mode: ChatSpeechMode::Say,
                        text: format!("E2EF4 mute {index}"),
                    })
                    .await?;
                last = Some((result.disposition, result.wait_seconds));
                if result.disposition == ChatDisposition::Muted && result.wait_seconds > 0 {
                    row.pass("mute", "native-spam-gate-muted-with-positive-wait");
                    break;
                }
            }
            Ok::<_, oteryn_dev_client::DevClientError>(last)
        })
        .await?;
        if !row.passed.contains("mute") {
            failures.push(format!("mute: 64 rapid says never muted; last={last:?}"));
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures.join(" | ").into())
        }
    }
    .await;
    let (cleanup_result, cleanup, attempts, busy_timed_out) =
        match while_serving(client, super::logout_retrying_busy(&mut peer)).await {
            Ok(cleanup) => cleanup,
            Err(error) => return result.and(Err(error)),
        };
    if cleanup_result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("speech peer cleanup logout refused: {cleanup}; attempts={attempts}; busy_timed_out={busy_timed_out}; journey={result:?}").into());
    }
    result
}

#[cfg(test)]
mod shop_tests {
    use super::sell_all_matches;

    #[test]
    fn rejects_partial_sell_all_even_when_gold_matches() {
        assert!(!sell_all_matches(2, 1, 7, 7));
        assert!(sell_all_matches(2, 2, 14, 7));
    }
}

#[cfg(test)]
mod trade_tests {
    use super::race_custody_matches;
    use oteryn_protocol_oteryn::player_trade::ResultCode::*;

    #[test]
    fn race_requires_a_closed_trade_with_matching_disposition_and_custody() {
        assert!(race_custody_matches(Completed, false, true, true));
        assert!(race_custody_matches(Stale, true, false, true));
        assert!(!race_custody_matches(Rejected, true, false, true));
        assert!(!race_custody_matches(Completed, true, false, true));
        assert!(!race_custody_matches(Stale, true, false, false));
    }

    #[test]
    fn equal_trade_lots_cannot_prove_a_swap() {
        assert!(!distinguishable_lots((46, 1), (46, 1)));
        assert!(distinguishable_lots((46, 1), (47, 1)));
        assert!(distinguishable_lots((46, 1), (46, 2)));
    }

    use super::distinguishable_lots;
}

#[cfg(test)]
mod position_tests {
    use super::*;

    #[test]
    fn spawn_alone_cannot_prove_position_persistence() {
        let spawn = TilePos {
            x: 10,
            y: 20,
            floor: -7,
        };
        let moved = TilePos { x: 11, ..spawn };
        assert!(!saved_logout_position(spawn, spawn, spawn));
        assert!(!saved_logout_position(spawn, moved, spawn));
        assert!(saved_logout_position(spawn, moved, moved));
    }
}

#[cfg(test)]
mod idle_tests {
    use super::*;

    #[test]
    fn idle_release_requires_timed_warning_and_terminal_logout() {
        let valid = |warning, removal, eof, released| {
            idle_removal_evidence(
                Some(Duration::from_secs(warning)),
                Duration::from_secs(removal),
                Duration::from_secs(2),
                eof,
                released,
            )
        };
        assert!(valid(902, 963, true, true));
        assert!(!valid(3, 963, true, true));
        assert!(!valid(902, 904, true, true));
        assert!(!valid(902, 963, false, true));
        assert!(!valid(902, 963, true, false));
    }
}

#[cfg(test)]
mod speech_tests {
    use super::*;
    use oteryn_dev_client::{ChatLine, ChatSpeaker, ChatSpeechMode};

    #[test]
    fn whisper_requires_the_current_command_and_exact_speaker() -> TestResult {
        let speaker = ChatSpeaker {
            identity: [1; 16],
            generation: 1u64.try_into()?,
        };
        let impostor = ChatSpeaker {
            identity: [2; 16],
            ..speaker
        };
        let line = ChatLine::Local {
            speaker,
            speaker_name: "Speaker".into(),
            mode: ChatSpeechMode::Whisper,
            text: "pspsps".into(),
            position: ActorPosition {
                x: 20,
                y: 20,
                floor: 7,
            },
        };
        let matches = |sequence, expected| {
            speech_matches(
                &line,
                sequence,
                20,
                expected,
                ChatSpeechMode::Whisper,
                "pspsps",
            )
        };
        assert!(matches(21, speaker));
        assert!(!matches(20, speaker));
        assert!(!matches(21, impostor));
        Ok(())
    }
}

#[cfg(test)]
mod coin_plan_tests {
    use super::*;

    #[test]
    fn coin_steps_use_an_independently_funded_crystal_before_platinum_upgrade() {
        let mut counts = [100_u64, 0, 0];
        for (action, _, _, spend, spent, gain, gained) in coin_steps() {
            if action == "crystal-down" {
                counts[2] += 1;
            }
            assert!(counts[spend] >= spent, "{action}: {counts:?}");
            counts[spend] -= spent;
            counts[gain] += gained;
        }
        assert_eq!(counts, [100, 0, 1]);
    }
}

#[cfg(test)]
mod eligibility_tests {
    use super::*;
    use oteryn_dev_client::ChatDisposition;

    #[test]
    fn converter_cannot_pass_without_observed_charge_or_retirement() {
        assert!(converter_charge_matches(500, Some(499), false));
        assert!(converter_charge_matches(1, None, true));
        assert!(!converter_charge_matches(500, Some(500), false));
        assert!(!converter_charge_matches(2, None, true));
    }

    #[test]
    fn yell_matrix_requires_typed_level_refusal_and_eligible_delivery() {
        assert!(yell_matrix_matches(
            ChatDisposition::LevelTooLow,
            ChatDisposition::Ok
        ));
        assert!(!yell_matrix_matches(
            ChatDisposition::Muted,
            ChatDisposition::Ok
        ));
        assert!(!yell_matrix_matches(
            ChatDisposition::Ok,
            ChatDisposition::Ok
        ));
    }
}

#[cfg(test)]
mod speech_range_tests {
    use super::*;

    #[test]
    fn far_say_requires_the_walk_to_reach_the_range_from_the_actual_origin() {
        let listener = TilePos {
            x: 100,
            y: 100,
            floor: -7,
        };
        assert!(!far_speech_position(
            TilePos { x: 107, ..listener },
            listener
        ));
        assert!(far_speech_position(
            TilePos { x: 110, ..listener },
            listener
        ));
    }
}

#[test]
fn full_gold_lot_requires_more_than_equal_value() {
    let picks_full_gold = coin_steps()[0].2;
    assert!(![99, 1].into_iter().any(picks_full_gold));
    assert!(picks_full_gold(100));
}

#[test]
fn trade_rejects_an_equal_spare_in_place_of_the_offered_instance() {
    let mine = vec![vec![1], vec![2]];
    let theirs = vec![vec![3]];
    let wrong_mine = vec![vec![1], vec![3]];
    let wrong_theirs = vec![vec![2]];
    let offered = [&mine[0], &theirs[0]];
    assert!(!exact_trade_custody(
        [&mine, &theirs],
        [&wrong_mine, &wrong_theirs],
        offered,
        true
    ));
    assert!(exact_trade_custody(
        [&mine, &theirs],
        [&[vec![2], vec![3]], &[vec![1]]],
        offered,
        true
    ));
}
