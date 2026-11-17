//! E2EF4: real-world economy/social journeys, with fail-closed full-row evidence.
//! Every step is a native client command a player could send. No fixture provisioning, SQL
//! mutations, teleports, clock overrides or invented wire handles; a step the server refuses
//! fails the row with the server's own refusal as the GAP text.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityRef};
use oteryn_protocol_oteryn::npc_service as wire;

use verdict::Row;

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
            let result = if let Some(active) = client.as_mut() {
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
                    // An interrupted command or frame cannot be resumed by another row.
                    *client = None;
                }
                result
            } else {
                Err("prior-native-journey-deadline;connection-closed;actions-unproven".into())
            };
            row.finish(&result);
        }};
    }
    row!(600, "E01", E01, shop, clients, world, npcs);
    row!(180, "E03", E03, coins, clients);
    row!(240, "E04", E04, trade, clients, world);
    row!(90, "E05", E05, market, world);
    row!(240, "E07", E07, storage, clients, world);
    row!(90, "S01", S01, party, clients, world);
    row!(90, "S03", S03, guild);
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

fn own(client: &DevClientSession) -> TestResult<EntityRef> {
    let entities = client.world_entities().ok_or("native entities absent")?;
    entities
        .iter()
        .find(|entity| &entity.entity.identity == entities.own_identity())
        .map(|entity| entity.entity)
        .ok_or_else(|| "native own actor absent".into())
}

async fn approach_peer(
    client: &mut DevClientSession,
    peer: &DevClientSession,
    world: &BundleWorld,
) -> TestResult {
    let other = tile(peer.world_spatial().actor_position)?;
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
    client: &DevClientSession,
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
    // The transfer recipient is a real second player, online and visible in town.
    let mut peer = connect_character(clients, 0x87, 0).await?;
    let result = shop_journey(row, client, &mut peer, clients, world, npcs).await;
    let (cleanup_result, cleanup, attempts, busy_timed_out) =
        super::logout_retrying_busy(&mut peer).await?;
    if cleanup_result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("shop peer cleanup logout refused: {cleanup}; attempts={attempts}; busy_timed_out={busy_timed_out}; journey={result:?}").into());
    }
    result
}

async fn shop_journey(
    row: &mut Row,
    client: &mut DevClientSession,
    peer: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult {
    use oteryn_dev_client::client_runtime::npc_shop as runtime;
    use oteryn_protocol_oteryn::npc_shop as shop;
    let refs = coin_refs()?;
    let recipient = own_name(peer)?;
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
                && carried.get(&o.item_definition_ref).copied().unwrap_or(0) >= 1
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
    *sold -= 1;
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
    let present = before.get(&offer.item_definition_ref).copied().unwrap_or(0);
    if sold_units == 0
        || sold_units > present
        || outcome.total_gold_units != sold_units * u64::from(offer.unit_price)
    {
        return Err(format!(
            "SELL_ALL result disagrees with the quote: sold={sold_units} present={present} gold={} unit_price={}",
            outcome.total_gold_units, offer.unit_price
        )
        .into());
    }
    let mut expected_items = non_coin(&before, &refs);
    let left = present - sold_units;
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
    *expected_items.entry(offer.item_definition_ref).or_default() += 1;
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

    // The recipient reads its own balance at the same banker before and after.
    let peer_actor = approach_npc(peer, world, banker).await?;
    speak(peer, peer_actor, "hi").await?;
    let peer_before = bank_balance(peer, peer_actor).await?;
    bank_exchange(
        client,
        bank,
        &format!("transfer 1 gold to {recipient}"),
        &format!("Do you want to transfer 1 gold to {recipient}?"),
        &format!("Very well. You have transferred 1 gold to {recipient}."),
    )
    .await?;
    let after_transfer = bank_balance(client, bank).await?;
    let peer_after = bank_balance(peer, peer_actor).await?;
    if after_transfer != after_withdraw - 1 || peer_after != peer_before + 1 {
        return Err(format!(
            "transfer: sender {after_withdraw}->{after_transfer}, recipient {peer_before}->{peer_after}"
        )
        .into());
    }
    row.pass("transfer", "sender-debited-and-real-recipient-credited");
    beat(row);

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
    Ok(())
}

async fn coins(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    let refs = coin_refs()?;
    let mut failures: Vec<String> = Vec::new();
    // (action, coin used, input quantity filter, coin stack spent, coin stack gained, gained)
    type Pick = fn(u32) -> bool;
    let steps: [(&'static str, usize, Pick, usize, u64, usize, u64); 4] = [
        ("gold-up", 0, |q| q >= 100, 0, 100, 1, 1),
        ("platinum-down", 1, |q| q != 100, 1, 1, 0, 100),
        ("platinum-up", 1, |q| q == 100, 1, 100, 2, 1),
        ("crystal-down", 2, |_| true, 2, 1, 1, 100),
    ];
    for (action, used, pick, spend, spent, gain, gained) in steps {
        let units = inventory_units(client)?;
        let before = coin_counts(&units, &refs);
        let item = client
            .inventory()
            .and_then(|inventory| {
                inventory
                    .entries
                    .iter()
                    .find(|i| i.item_definition_ref == refs[used] && pick(i.count.get()))
            })
            .copied();
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
    // Converter items have no player acquisition path in the town fixture; absent items fail
    // with the precondition, never with a synthetic grant.
    let definitions = provisioning::item_index()?;
    let mut converters = Vec::new();
    for id in [23722, 25719, 28525, 28526] {
        let reference = provisioning::item_reference(&definitions, id)?;
        converters.push((
            id,
            client
                .inventory()
                .is_some_and(|i| i.entries.iter().any(|e| e.item_definition_ref == reference)),
        ));
    }
    if converters.iter().all(|(_, carried)| !carried) {
        failures.push(format!(
            "converter-charge/converter-retirement/magic-toggle/magic-tick: no coin converter is carried ({converters:?}) and no town NPC offers one for earned gold"
        ));
    } else {
        failures.push(format!(
            "converter-charge/converter-retirement/magic-toggle/magic-tick: converters carried {converters:?} but their charge and tick assertions are not authored"
        ));
    }
    let carried = inventory_units(client)?;
    relog(client, clients).await?;
    if inventory_units(client)? != carried {
        failures.push(format!(
            "relog-coins-charges: carried custody changed across fresh admission: before={carried:?} after={:?}",
            inventory_units(client)?
        ));
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join(" | ").into())
    }
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
    let mut peer = connect_character(clients, 0x81, 0).await?;
    let result: TestResult = async {
        approach_peer(client, &peer, world).await?;
        let command = runtime::PlayerTradeCommand::new(runtime::player_trade::Intent::Offer {
            partner: own(&peer)?,
            item: item.handle.get(),
        })
        .ok_or("trade offer encoding")?;
        let reply = runtime::send(client.session_mut(), &command).await?;
        if reply.result != runtime::player_trade::ResultCode::Offered {
            return Err(format!("trade request: {:?}", reply.result).into());
        }
        row.pass("request", "real-peer-offer-opened");
        peer.session_mut().read_gated_delta(26, 1).await?;
        let id = reply.state.trade_id.ok_or("offer has no trade id")?;
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
        row.pass("cancel", "active-offer-closed");
        // The cancelled round invalidated the first handle; read the leaf again.
        let item = client
            .inventory()
            .and_then(|i| i.entries.first())
            .copied()
            .ok_or("no direct-backpack leaf left for the second round")?;
        let mine = inventory_units(client)?;
        let theirs = inventory_units(&peer)?;
        let peer_item = peer
            .inventory()
            .and_then(|i| i.entries.first())
            .copied()
            .ok_or("peer has no direct-backpack leaf to counter-offer")?;
        // Round two: both players offer, both accept, custody swaps exactly.
        let offer = runtime::PlayerTradeCommand::new(runtime::player_trade::Intent::Offer {
            partner: own(&peer)?,
            item: item.handle.get(),
        })
        .ok_or("second offer encoding")?;
        let reply = runtime::send(client.session_mut(), &offer).await?;
        if reply.result != runtime::player_trade::ResultCode::Offered {
            return Err(format!("second offer: {:?}", reply.result).into());
        }
        let id = reply.state.trade_id.ok_or("second offer has no trade id")?;
        let counter = runtime::PlayerTradeCommand::new(runtime::player_trade::Intent::Offer {
            partner: own(client)?,
            item: peer_item.handle.get(),
        })
        .ok_or("counter offer encoding")?;
        let reply = runtime::send(peer.session_mut(), &counter).await?;
        if reply.result != runtime::player_trade::ResultCode::Offered && reply.result != runtime::player_trade::ResultCode::Accepted {
            return Err(format!("peer counter-offer: {:?}", reply.result).into());
        }
        let accept = runtime::PlayerTradeCommand::new(runtime::player_trade::Intent::Accept { trade_id: id })
            .ok_or("accept encoding")?;
        let first = runtime::send(client.session_mut(), &accept).await?;
        if first.result != runtime::player_trade::ResultCode::Accepted {
            return Err(format!("first accept: {:?}", first.result).into());
        }
        let second = runtime::send(peer.session_mut(), &accept).await?;
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
        settle(client, "trade-accept", |now| *now == client_after).await?;
        settle(&mut peer, "trade-accept-peer", |now| *now == peer_after).await?;
        row.pass("accept", "both-accepts-swapped-custody-exactly");
        beat(row);
        // Round three: the owner logs out while the partner accepts; the trade can never
        // complete without the owner's own accept, so custody is unchanged on both sides.
        let item = client
            .inventory()
            .and_then(|i| i.entries.first())
            .copied()
            .ok_or("no leaf for the logout race")?;
        let peer_item = peer
            .inventory()
            .and_then(|i| i.entries.first())
            .copied()
            .ok_or("peer has no leaf for the logout race")?;
        let before_client = inventory_units(client)?;
        let before_peer = inventory_units(&peer)?;
        let offer = runtime::PlayerTradeCommand::new(Intent::Offer {
            partner: own(&peer)?,
            item: item.handle.get(),
        })
        .ok_or("race offer encoding")?;
        let reply = runtime::send(client.session_mut(), &offer).await?;
        if reply.result != ResultCode::Offered {
            return Err(format!("race offer: {:?}", reply.result).into());
        }
        let id = reply.state.trade_id.ok_or("race offer has no trade id")?;
        let counter = runtime::PlayerTradeCommand::new(Intent::Offer {
            partner: own(client)?,
            item: peer_item.handle.get(),
        })
        .ok_or("race counter encoding")?;
        runtime::send(peer.session_mut(), &counter).await?;
        let accept = runtime::PlayerTradeCommand::new(runtime::player_trade::Intent::Accept {
            trade_id: id,
        })
        .ok_or("race accept encoding")?;
        let first = runtime::send(client.session_mut(), &accept).await?;
        if first.result != runtime::player_trade::ResultCode::Accepted {
            return Err(format!("owner accept before the race: {:?}", first.result).into());
        }
        let (accepted, logout) = tokio::join!(
            runtime::send(peer.session_mut(), &accept),
            super::logout_retrying_busy(client)
        );
        let (logout_result, logout_text, attempts, busy_timed_out) = logout?;
        if logout_result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Err(format!("race logout refused: {logout_text}; attempts={attempts}; busy_timed_out={busy_timed_out}; peer accept={:?}", accepted.map(|r| r.result)).into());
        }
        row.pass("logout-race", "partner-accept-raced-owner-logout-without-error");
        *client = connect_character(clients, 0x85, 1).await?;
        let a = (item.item_definition_ref.get(), u64::from(item.count.get()));
        let b = (peer_item.item_definition_ref.get(), u64::from(peer_item.count.get()));
        let swapped_client = swap(&before_client, a.0, b.0, a.1, b.1);
        let swapped_peer = swap(&before_peer, b.0, a.0, b.1, a.1);
        let now_client = settle(client, "race-relog", |now| {
            *now == before_client || *now == swapped_client
        })
        .await?;
        let completed = now_client == swapped_client && now_client != before_client;
        let (expected_peer, side) = if completed {
            (&swapped_peer, "completed-swap")
        } else {
            (&before_peer, "cancelled-unchanged")
        };
        settle(&mut peer, "race-peer", |now| now == expected_peer).await?;
        row.pass(
            "relog-custody",
            format!("fresh-admission-custody-exactly-{side}-on-both-sides"),
        );
        Ok(())
    }
    .await;
    let (cleanup_result, cleanup, attempts, busy_timed_out) =
        super::logout_retrying_busy(&mut peer).await?;
    if cleanup_result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("trade peer cleanup logout refused: {cleanup}; attempts={attempts}; busy_timed_out={busy_timed_out}; journey={result:?}").into());
    }
    result
}

async fn market(row: &mut Row, client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    use oteryn_dev_client::{client_runtime::market as runtime, client_ui::market as ui};
    locker(client, world).await?;
    let mut panel = ui::MarketPanel {
        selected: true,
        ..Default::default()
    };
    for (intent, action) in [
        (ui::Intent::Browse, "browse"),
        (ui::Intent::History, "history"),
    ] {
        let command = ui::request(&mut panel, intent).ok_or("market request input")?;
        let reply = runtime::send(client.session_mut(), &command).await?;
        if !ui::apply_result(&mut panel, &reply) {
            return Err(format!("market {action}: {:?}", reply.reply).into());
        }
        // Empty history is a successful query, but not settlement history evidence.
        if action == "browse" {
            row.pass("browse", "native-market-browse-decoded");
        }
    }
    let item = client
        .inventory()
        .and_then(|i| i.entries.first().or(i.main_backpack.as_ref()))
        .copied()
        .ok_or("no native item definition for market offer")?;
    panel.definition = item.item_definition_ref.get();
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
    Err("BLOCKED market accept/history/relog-settlement: not authored; needs a second player funded by real sales to accept the offer and the history/relog reads".into())
}

async fn depot_call(
    client: &mut DevClientSession,
    town: Option<u16>,
    action: oteryn_protocol_oteryn::depot::DepotAction,
) -> TestResult<oteryn_protocol_oteryn::depot::DepotReply> {
    use oteryn_protocol_oteryn::depot;
    let payload = depot::encode_depot_intent(depot::DepotIntent {
        town,
        box_number: 1,
        page: 0,
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
    depot::decode_depot_reply(&reply.payload).map_err(|e| format!("depot reply: {e:?}").into())
}

async fn storage(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    use oteryn_dev_client::{client_runtime::stash as runtime, client_ui::stash as ui};
    use oteryn_protocol_oteryn::depot;
    locker(client, world).await?;
    for box_number in [1] {
        let payload = depot::encode_depot_intent(depot::DepotIntent {
            town: None,
            box_number,
            page: 0,
            action: depot::DepotAction::Browse,
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
        let reply =
            depot::decode_depot_reply(&reply.payload).map_err(|e| format!("depot reply: {e:?}"))?;
        if reply.outcome != depot::DepotOutcome::Opened
            || reply.box_number != box_number
            || reply.town.is_none()
        {
            return Err(format!("depot/inbox browse: {reply:?}").into());
        }
        // Open views alone do not prove depot/inbox custody.
    }
    // Depot custody round trip: deposit a whole carried stack, prove the locker holds exactly it,
    // then withdraw that locker entry by ordinal and prove the backpack is whole again.
    {
        let leaf = client
            .inventory()
            .and_then(|i| i.entries.first())
            .copied()
            .ok_or("no direct-backpack leaf to deposit into the depot")?;
        let whole = inventory_units(client)?;
        let town = {
            let browse = depot_call(client, None, depot::DepotAction::Browse).await?;
            browse.town.ok_or("depot browse named no town")?
        };
        let held_before = depot_call(client, Some(town), depot::DepotAction::Browse)
            .await?
            .entries
            .iter()
            .filter(|e| e.definition == leaf.item_definition_ref.get())
            .map(|e| u64::from(e.count))
            .sum::<u64>();
        let moved =
            depot_call(client, Some(town), depot::DepotAction::Deposit(leaf.handle)).await?;
        if moved.outcome != depot::DepotOutcome::Moved {
            return Err(format!("depot deposit refused: {moved:?}").into());
        }
        let mut expected = whole.clone();
        let units = expected
            .get_mut(&leaf.item_definition_ref.get())
            .ok_or("deposited definition absent")?;
        *units -= u64::from(leaf.count.get());
        if *units == 0 {
            expected.remove(&leaf.item_definition_ref.get());
        }
        wait_inventory(client, &expected).await?;
        let listing = depot_call(client, Some(town), depot::DepotAction::Browse).await?;
        let held_after = listing
            .entries
            .iter()
            .filter(|e| e.definition == leaf.item_definition_ref.get())
            .map(|e| u64::from(e.count))
            .sum::<u64>();
        if held_after != held_before + u64::from(leaf.count.get()) {
            return Err(format!(
                "depot holds {held_after} after depositing {} on top of {held_before}",
                leaf.count.get()
            )
            .into());
        }
        let ordinal = listing
            .entries
            .iter()
            .find(|e| e.definition == leaf.item_definition_ref.get())
            .map(|e| e.ordinal)
            .ok_or("deposited entry absent from the depot listing")?;
        let moved = depot_call(client, Some(town), depot::DepotAction::Withdraw(ordinal)).await?;
        if moved.outcome != depot::DepotOutcome::Moved {
            return Err(format!("depot withdraw refused: {moved:?}").into());
        }
        settle(client, "depot-withdraw", |now| *now == whole).await?;
        let after = depot_call(client, Some(town), depot::DepotAction::Browse).await?;
        let held_end = after
            .entries
            .iter()
            .filter(|e| e.definition == leaf.item_definition_ref.get())
            .map(|e| u64::from(e.count))
            .sum::<u64>();
        if held_end != held_before {
            return Err(format!(
                "depot still holds {held_end} after the withdraw; started with {held_before}"
            )
            .into());
        }
        row.pass("depot", "deposit-exact-stack-then-withdraw-by-ordinal");
    }
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
    row.pass(
        "relog-custody",
        "fresh-admission-inventory-and-stash-quantities-retained",
    );
    Err(
        "inbox: retrieving a parcel needs a labelled parcel posted by another player to this character's mailbox (MAIL_V1 capability); that send flow is not authored; box 17 is not treated as an inbox"
            .into(),
    )
}

async fn party(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
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
    let mut peer = connect_character(clients, 0x82, 0).await?;
    let result: TestResult = async {
        approach_peer(client, &peer, world).await?;
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
        send(client, Some(PartyIntent::Leave)).await?;
        if !send(client, None).await?.members.is_empty() {
            return Err("party leave retained membership".into());
        }
        row.pass("leave", "leaver-current-query-empty");
        Err("BLOCKED shared-xp-split: not authored; needs a real kill by both members with an XP observation per member".into())
    }
    .await;
    let _ = send(client, Some(PartyIntent::Leave)).await;
    let (cleanup_result, cleanup, attempts, busy_timed_out) =
        super::logout_retrying_busy(&mut peer).await?;
    if cleanup_result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("party peer cleanup logout refused: {cleanup}; attempts={attempts}; busy_timed_out={busy_timed_out}; journey={result:?}").into());
    }
    result
}

async fn guild(row: &mut Row, client: &mut DevClientSession) -> TestResult {
    use oteryn_dev_client::client_runtime::guild as runtime;
    row.missing_server = Some("source:crates/protocol-oteryn/src/guild.rs:17");
    let reply = runtime::send(
        client.session_mut(),
        &runtime::GuildCommand::Query(runtime::guild::GuildQuery::Own),
    )
    .await?;
    // Native GuildIntent has membership/rank writers but no MOTD writer. Reading
    // GuildSummary.message cannot prove an update. Keep every other gap visible too.
    evidence(
        "E2EF4_MISSING S03 motd source:crates/protocol-oteryn/src/guild.rs:17 native-GuildIntent-has-no-MOTD-write",
    );
    Err(format!("guild-query={reply:?};BLOCKED guild/ranks/chat/bank/wars/relog: not authored (S03 is outside packet E2E-G1);MOTD-write-unavailable").into())
}

async fn logout(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    let mut peer = connect_character(clients, 0x83, 0).await?;
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
    let retained = peer.world_spatial().actor_position == position
        && peer.join_snapshot().game_session_id != session;
    let (cleanup_result, cleanup, attempts, busy_timed_out) =
        super::logout_retrying_busy(&mut peer).await?;
    if !retained || cleanup_result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("fresh-login-position-or-cleanup-mismatch: retained={retained}; cleanup={cleanup}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
    }
    row.pass("relog-position", "fresh-session-retained-saved-position");
    let mut failures: Vec<String> = Vec::new();
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
    // Idle: the server's idle policy runs on the real monotonic clock (warning at 15 minutes,
    // removal at 16), so this is a real wait. The peer stays silent; the main client acts once
    // at 14 minutes and must not be warned at 15.
    let idle: TestResult = async {
        let mut silent = connect_character(clients, 0x8c, 0).await?;
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
                walk(client, world, &path).await?;
                if anchored {
                    stepped = true;
                }
                anchored = true;
            }
            match silent.service_liveness(Duration::from_secs(1)).await {
                Ok(_) => {
                    if warned_at.is_none()
                        && notices(&mut silent).iter().any(|t| {
                            t.starts_with("There was no variation in your behaviour for 15 minutes")
                        })
                    {
                        warned_at = Some(start.elapsed());
                    }
                }
                Err(_) => removed_at = Some(start.elapsed()),
            }
            if client
                .service_liveness(Duration::from_millis(10))
                .await
                .is_ok()
                && notices(client)
                    .iter()
                    .any(|t| t.starts_with("There was no variation in your behaviour"))
            {
                warned_actor = true;
            }
            beat(row);
        }
        if removed_at.is_none() {
            let (done, text, ..) = super::logout_retrying_busy(&mut silent).await?;
            if done != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
                return Err(format!(
                    "silent peer was never removed and its logout was refused: {text}"
                )
                .into());
            }
        }
        let mut errors: Vec<String> = Vec::new();
        match warned_at {
            Some(at) => row.pass("idle-warning", format!("warned-at-{}s", at.as_secs())),
            None => {
                errors.push("idle-warning: the silent peer received no 15-minute warning".into())
            }
        }
        match removed_at {
            Some(at) => {
                let reconnect: TestResult<_> = async {
                    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
                    loop {
                        match connect_character(clients, 0x8d, 0).await {
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
                                break Ok(id);
                            }
                            Err(error) if tokio::time::Instant::now() >= deadline => {
                                break Err(error);
                            }
                            Err(_) => tokio::time::sleep(Duration::from_millis(500)).await,
                        }
                    }
                }
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

fn heard(listener: &DevClientSession, mode: oteryn_dev_client::ChatSpeechMode, text: &str) -> bool {
    listener.chat_log().is_some_and(|log| {
        log.lines().any(|line| {
            matches!(
                line,
                oteryn_dev_client::ChatLine::Local { mode: seen, text: said, .. }
                    if *seen == mode && said == text
            )
        })
    })
}

async fn listen(
    listener: &mut DevClientSession,
    mode: oteryn_dev_client::ChatSpeechMode,
    text: &str,
    window: Duration,
) -> TestResult<bool> {
    let deadline = tokio::time::Instant::now() + window;
    loop {
        if heard(listener, mode, text) {
            return Ok(true);
        }
        if tokio::time::Instant::now() >= deadline {
            return Ok(false);
        }
        progress();
        listener.service_liveness(Duration::from_millis(20)).await?;
        listener.take_events();
    }
}

async fn speech(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    use oteryn_dev_client::{ChatDisposition, ChatIntent, ChatSpeechMode};
    // Recipients are a real second player; every spoken line is read on the listener's own log.
    let mut peer = connect_character(clients, 0x88, 0).await?;
    let result: TestResult = async {
        let mut failures: Vec<String> = Vec::new();
        approach_peer(client, &peer, world).await?;
        let window = Duration::from_secs(5);
        let mut yell_client = None;
        for (action, mode) in [
            ("say", ChatSpeechMode::Say),
            ("whisper", ChatSpeechMode::Whisper),
            ("yell", ChatSpeechMode::Yell),
        ] {
            tokio::time::sleep(Duration::from_millis(1200)).await;
            let text = format!("E2EF4 {action}");
            let result = client
                .chat(&ChatIntent::Say {
                    mode,
                    text: text.clone(),
                })
                .await?;
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
            let expected = if mode == ChatSpeechMode::Yell {
                text.to_uppercase()
            } else {
                text
            };
            if !listen(client, mode, &expected, window).await? {
                failures.push(format!("{action}: own local line absent"));
            } else if !listen(&mut peer, mode, &expected, window).await? {
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
        let far = route(world, tile(client.world_spatial().actor_position)?, |at| {
            at.floor == other.floor && at.x.abs_diff(other.x) >= 10 && at.y.abs_diff(other.y) <= 2
        });
        let mut range_proof = Vec::new();
        match near {
            Some(path) => {
                walk(client, world, &path).await?;
                tokio::time::sleep(Duration::from_millis(1200)).await;
                let result = client
                    .chat(&ChatIntent::Say {
                        mode: ChatSpeechMode::Whisper,
                        text: "E2EF4 range whisper".into(),
                    })
                    .await?;
                if result.disposition != ChatDisposition::Ok {
                    failures.push(format!("range: whisper refused: {:?}", result.disposition));
                } else if listen(&mut peer, ChatSpeechMode::Whisper, "pspsps", window).await? {
                    range_proof.push("whisper-obscured-at-distance");
                } else {
                    failures.push("range: distant whisper was not delivered obscured as pspsps".into());
                }
            }
            None => failures.push("range: no authored tile 3-5 columns from the peer".into()),
        }
        match far {
            Some(path) => {
                walk(client, world, &path).await?;
                tokio::time::sleep(Duration::from_millis(1200)).await;
                let text = "E2EF4 range say";
                let result = client
                    .chat(&ChatIntent::Say {
                        mode: ChatSpeechMode::Say,
                        text: text.into(),
                    })
                    .await?;
                if result.disposition != ChatDisposition::Ok {
                    failures.push(format!("range: far say refused: {:?}", result.disposition));
                } else if !listen(client, ChatSpeechMode::Say, text, window).await? {
                    failures.push("range: far say has no own local line".into());
                } else if listen(&mut peer, ChatSpeechMode::Say, text, Duration::from_secs(2)).await? {
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
        // Yell level/premium matrix: the same refusal rule for both real players.
        tokio::time::sleep(Duration::from_millis(1200)).await;
        let yell_peer = peer
            .chat(&ChatIntent::Say {
                mode: ChatSpeechMode::Yell,
                text: "E2EF4 peer yell".into(),
            })
            .await?
            .disposition;
        // The matrix needs one refused ineligible speaker and one accepted eligible speaker.
        // Two starter characters are both level 1 and non-premium, so it can never be proven
        // here; two accepted yells would show the gate is missing, not that it holds.
        failures.push(format!(
            "yell-level-premium: yell dispositions client={yell_client:?} peer={yell_peer:?}; the matrix needs a level 20 or premium character that real play cannot reach in node boot"
        ));
        tokio::time::sleep(Duration::from_millis(1200)).await;
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
        super::logout_retrying_busy(&mut peer).await?;
    if cleanup_result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("speech peer cleanup logout refused: {cleanup}; attempts={attempts}; busy_timed_out={busy_timed_out}; journey={result:?}").into());
    }
    result
}
